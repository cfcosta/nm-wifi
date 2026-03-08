{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
    {
      pre-commit-hooks,
      rust-overlay,
      treefmt-nix,
      ...
    }@inputs:
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
            nativeBuildInputs = [ pkgs.pkg-config ];
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
              rustfmt.package = rust;
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

      checks = forEachSupportedSystem (
        { system, ... }:
        {
          pre-commit-check = pre-commit-hooks.lib.${system}.run {
            src = ./.;

            hooks = {
              deadnix.enable = true;
              nixfmt-rfc-style.enable = true;
              treefmt = {
                enable = true;
                package = inputs.self.formatter.${system};
              };
            };
          };
        }
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
