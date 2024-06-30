{
  description = "A Rust web server including a NixOS module";
  # Nixpkgs / NixOS version to use.
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crate2nix.url = "github:nix-community/crate2nix";
  };
  outputs = { self, nixpkgs, crate2nix }:
    let
      # to work with older version of flakes
      lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
      # Generate a user-friendly version number.
      version = "${builtins.substring 0 8 lastModifiedDate}-${self.shortRev or "dirty"}";
      # System types to support.
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      # Helper function to generate an attrset '{ x86_64-linux = f "x86_64-linux"; ... }'.
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
      # Nixpkgs instantiated for supported system types.
      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; overlays = [ self.overlays.default ]; });
    in
    {
      # A Nixpkgs overlay.
      overlays.default = final: prev: {
        mc_discord_bot = with final; final.callPackage
          ({ inShell ? false }:
            let
              generated = crate2nix.tools.${stdenv.hostPlatform.system}.generatedCargoNix {
                name = "mc_discord_bot";
                src = ./.;
              };
              crates = pkgs.callPackage "${generated}/default.nix" {
                buildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override {
                  defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                    mc_discord_bot = attrs: {
                      buildInputs = with final.darwin.apple_sdk.frameworks; [] ++ lib.optionals (stdenv.isDarwin) [
                        SystemConfiguration
                        CoreServices
                      ];
                      nativeBuildInputs = with pkgs; [ libiconv pkg-config openssl ];
                      # Needed to get openssl-sys to use pkg-config.
                      OPENSSL_NO_VENDOR = 1;
                      OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
                      OPENSSL_DIR = "${lib.getDev pkgs.openssl}";
                    };
                  };
                };
              };
            in
            crates.workspaceMembers.mc_discord_bot.build
            )
          { };
      };
      # Provide some binary packages for selected system types.
      packages = forAllSystems (system:
        {
          inherit (nixpkgsFor.${system}) mc_discord_bot;
          # The default package for 'nix build'. This makes sense if the
          # flake provides only one package or there is a clear "main"
          # package.
          default = self.packages.${system}.mc_discord_bot;
        });
      # Provide a 'nix develop' environment for interactive hacking.
      devShells = forAllSystems (system:
        {
          default = self.packages.${system}.mc_discord_bot.override { inShell = true; };
        });
      # A NixOS module.
      nixosModules.mc_discord_bot =
        { pkgs, lib, config, ... }:
        {
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
}
