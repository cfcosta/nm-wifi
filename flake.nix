{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { rust-overlay, treefmt-nix, ... }@inputs:
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
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ (import rust-overlay) ];
            };
            rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          in
          f { inherit pkgs rust system; }
        );
    in
    {
      packages = forEachSupportedSystem (
        { pkgs, rust, ... }:
        let
          rustPlatform = pkgs.makeRustPlatform {
            rustc = rust;
            cargo = rust;
          };
        in
        {
          default = rustPlatform.buildRustPackage {
            name = "nm-wifi";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "--bin"
              "nm-wifi"
            ];
            buildInputs = [ pkgs.dbus.dev ];
            # `remove-references-to` (used in postInstall) strips the rust
            # toolchain path that rustc bakes into binary debug info via
            # `rust-src`. Without it the runtime closure drags in the entire
            # nightly toolchain (rustc/cargo/rust-src/rust-docs/…), ~2 GiB,
            # into anything depending on this package.
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.removeReferencesTo
            ];
            postInstall = ''
              for bin in "$out"/bin/*; do
                remove-references-to -t ${rust} "$bin"
              done
            '';
            # Guard: fail the build if a rust toolchain reference survives, so
            # this closure leak can't silently return.
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
