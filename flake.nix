{
  description = "mc_discord_bot, A Rust web server including a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
    crate2nix.url = "github:nix-community/crate2nix";

    # Development

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "eigenvalue.cachix.org-1:ykerQDDa55PGxU25CETy9wF6uVDpadGGXYrFNJA3TUs=";
    extra-substituters = "https://eigenvalue.cachix.org";
    allow-import-from-derivation = true;
  };

  outputs = inputs @ { self, nixpkgs, flake-parts, rust-overlay, crate2nix, devshell, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    imports = [
      devshell.flakeModule
      flake-parts.flakeModules.easyOverlay
    ];

    flake = {
      nixosModules.mc_discord_bot = { pkgs, lib, config, ... }: {
        options.services.mc_discord_bot = {
          enable = lib.mkEnableOption "enable minecraft discord bot";
          env_file = lib.mkOption {
            type = lib.types.path;
            description = ''Path to the environment file, to be piped through xargs, must include the following variables:
              DISCORD_CLIENT_ID
              DISCORD_CLIENT_SECRET
              DISCORD_PUBLIC_KEY
              DISCORD_BOT_TOKEN
            '';
          };
        };
        config = lib.mkIf config.services.mc_discord_bot.enable {
          nixpkgs.overlays = [ self.overlays.default ];
          systemd.services = {
            mc_discord_bot = {
              wantedBy = [ "multi-user.target" ];
              script = ''
                export $(cat ${config.services.mc_discord_bot.env_file} | xargs)
                ${pkgs.mc_discord_bot}/bin/mc_discord_bot
              '';
            };
          };
        };
      };
    };

    perSystem = { system, pkgs, lib, config, ... }:
      let
        generatedCargoNix = crate2nix.tools.${system}.generatedCargoNix {
          name = "mc_discord_bot";
          src = ./.;
        };
        cargoNix = pkgs.callPackage "${generatedCargoNix}/default.nix" {
          buildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override {
            defaultCrateOverrides = pkgs.defaultCrateOverrides // {
              mc_discord_bot = attrs: {
                buildInputs = with pkgs.darwin.apple_sdk.frameworks; lib.optionals pkgs.stdenv.isDarwin [
                  SystemConfiguration
                  CoreServices
                ];
                nativeBuildInputs = with pkgs; [ libiconv pkg-config openssl ];
                OPENSSL_NO_VENDOR = 1;
                OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
                OPENSSL_DIR = "${lib.getDev pkgs.openssl}";
              };
            };
          };
        };
      in
      rec {
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
            (final: prev: assert !(prev ? rust-toolchain); rec {
              rust-toolchain = (prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                extensions = [ "rust-src" "rust-std" "rust-analyzer" "rustfmt" "clippy" ];
              };

              rustc = rust-toolchain;
              cargo = rust-toolchain;
              rustfmt = rust-toolchain;
              clippy = rust-toolchain;
              rust-analyzer = rust-toolchain;
            })
          ];
          config = { };
        };

        packages = {
          mc_discord_bot = cargoNix.workspaceMembers.mc_discord_bot.build;
          default = packages.mc_discord_bot;
        };
        overlayAttrs = {
          inherit (packages) mc_discord_bot;
        };
        devshells.default = {
          imports = [
            "${devshell}/extra/language/c.nix"
            # "${devshell}/extra/language/rust.nix"
          ];

          env = [
            {
              name = "RUST_LOG";
              value = "mc_discord_bot=trace";
            }
            {
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust-toolchain}/lib/rustlib/src/rust/library";
            }
          ];

          commands = with pkgs; [
            { package = rust-toolchain; category = "rust"; }
          ];

          language.c = {
            libraries = lib.optional pkgs.stdenv.isDarwin pkgs.libiconv;
          };
        };
      };
  };
}
