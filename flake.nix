{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nci.url = "github:yusdacra/nix-cargo-integration";
    # rust-overlay = {
    #   url = "github:oxalica/rust-overlay";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
  };

  outputs =
    inputs@{ self
    , flake-parts
    , nixpkgs
      # , rust-overlay
    , ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      imports = [ inputs.nci.flakeModule ];

      perSystem =
        { config
        , pkgs
        , system
        , ...
        }:
        let
          crate = config.nci.outputs.pixelbot;
        in
        {
          nci.projects.pixelbot = {
            path = ./.;
            export = true;
          };

          nci.crates.pixelbot = {
            export = true;
            drvConfig.mkDerivation = {
              nativeBuildInputs = with pkgs; [
                pkg-config
                openssl
              ];
            };
          };

          nci.toolchainConfig = {
            channel = "stable";
            components = [ "rust-analyzer" "rust-src" "clippy" "rustfmt" ];
          };

          # nci.toolchains.shell = (
          #   rust-overlay.packages.${system}.rust.override {
          #     extensions = [
          #       "cargo"
          #       "clippy"
          #       "rust-src"
          #       "rust-analyzer"
          #       "rustc"
          #       "rustfmt"
          #     ];
          #   }
          # );

          devShells.default = crate.devShell;

          packages = rec {
            pixelbot = crate.packages.release;
            default = pixelbot;
          };
        };

      flake.nixosModules = rec {
        spambotsen = import ./module.nix;
        default = spambotsen;
      };
    };
}
