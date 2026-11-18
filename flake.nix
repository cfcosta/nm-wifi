{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # The toolchain and the package builder: Rust's own release
    # manifests, and a builder that keeps compiled dependencies in a
    # derivation of their own.
    rust-dev-tools = {
      url = "github:cfcosta/rust-dev-tools.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { rust-dev-tools, treefmt-nix, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          let
            pkgs = import inputs.nixpkgs { inherit system; };
            # Stable from rust-toolchain.toml, with rustfmt from a pinned
            # nightly: the formatter's import options are nightly-only.
            # The linker stays the platform default.
            rdt = rust-dev-tools.setup pkgs {
              name = "nm-wifi";
              rust = rust-dev-tools.version.fromToolchainFile ./rust-toolchain.toml // {
                rustfmt = rust-dev-tools.version.fromToolchain "nightly" "2026-08-04";
              };
              overrides = {
                linux.useMold = false;
                darwin.useLLD = false;
              };
            };
            inherit (rdt) rust;
          in
          f {
            inherit
              pkgs
              rdt
              rust
              system
              ;
          }
        );
    in
    {
      packages = forEachSupportedSystem (
        {
          pkgs,
          rdt,
          rust,
          ...
        }:
        {
          default = rdt.buildRustPackage {
            name = "nm-wifi";
            src = ./.;
            buildInputs = [ pkgs.dbus.dev ];
            # rust-dev-tools remaps the toolchain's paths in panic locations
            # and debug info; fail the build if a reference survives anyway,
            # since it would pull the ~2 GiB toolchain into the closure.
            disallowedReferences = [ rust ];
            meta.mainProgram = "nm-wifi";
          };
        }
      );

      formatter = forEachSupportedSystem (
        { pkgs, rust, ... }:
        (treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";

          settings = {
            allow-missing-formatter = true;
            verbose = 0;

            global.excludes = [ "*.lock" ];

            formatter = {
              nixfmt.options = [ "--strict" ];

              rustfmt = {
                package = rust;
                options = [
                  "--config"
                  "reorder_imports=true,imports_granularity=Crate,imports_layout=HorizontalVertical,max_width=80,group_imports=StdExternalCrate,trailing_comma=Vertical,trailing_semicolon=true"
                ];
              };
            };
          };

          programs = {
            nixfmt.enable = true;
            oxfmt.enable = true;
            rustfmt = {
              enable = true;
              package = rust;
            };
            taplo.enable = true;
          };
        }).config.build.wrapper
      );

      devShells = forEachSupportedSystem (
        {
          pkgs,
          rust,
          system,
          ...
        }:
        {
          default = pkgs.mkShell {
            name = "nm-wifi";

            buildInputs = with pkgs; [
              rust
              inputs.self.formatter.${system}

              cargo-nextest
              cargo-mutants
              bacon

              pkg-config
              dbus.dev
            ];
          };
        }
      );
    };
}
