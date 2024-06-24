{
  description = "A Rust web server including a NixOS module";
  # Nixpkgs / NixOS version to use.
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, rust-overlay }:
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
      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; overlays = [ self.overlays.default (import rust-overlay) ]; });
    in
    {
      # A Nixpkgs overlay.
      overlays.default = final: prev: {
        mc-discord-bot = with final; final.callPackage
          ({ inShell ? false }:
            let
              rustToolchain = rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
                # In 'nix develop', provide some developer tools.
                extensions = [ "rust-src" "rust-std" ] ++ lib.optionals inShell [ "rust-analyzer" "rustfmt" "clippy" ];
              });
              rustPlatform = makeRustPlatform {
                cargo = rustToolchain;
                rustc = rustToolchain;
              };
              cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
            in
            rustPlatform.buildRustPackage {
              name = "${cargoToml.package.name}-${cargoToml.package.version}-${version}";
              pname = cargoToml.package.name;
              version = cargoToml.package.version;
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              nativeBuildInputs = [ pkg-config rustPlatform.bindgenHook openssl ];
              # Needed to get openssl-sys to use pkg-config.
              OPENSSL_NO_VENDOR = 1;
              OPENSSL_LIB_DIR = "${lib.getLib openssl}/lib";
              OPENSSL_DIR = "${lib.getDev openssl}";
            })
          { };
      };
      # Provide some binary packages for selected system types.
      packages = forAllSystems (system:
        {
          inherit (nixpkgsFor.${system}) mc-discord-bot;
          # The default package for 'nix build'. This makes sense if the
          # flake provides only one package or there is a clear "main"
          # package.
          default = self.packages.${system}.mc-discord-bot;
        });
      # Provide a 'nix develop' environment for interactive hacking.
      devShells = forAllSystems (system:
        {
          default = self.packages.${system}.mc-discord-bot.override { inShell = true; };
        });
      # A NixOS module.
      nixosModules.mc-discord-bot =
        { pkgs, lib, config, ... }:
        {
          options.services.mc-discord-bot.enable = lib.mkEnableOption "enable minecraft discord bot";
          config = lib.mkIf config.services.mc-discord-bot.enable {
            nixpkgs.overlays = [ self.overlays.default ];
            systemd.services = {
              mc-discord-bot = {
                wantedBy = [ "multi-user.target" ];
                serviceConfig.ExecStart = "${pkgs.mc-discord-bot}/bin/mc-discord-bot";
              };
            };
          };
        };
    };
}
