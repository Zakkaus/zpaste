{
  description = "Zpaste is a pastebin based on wastebin";

  # Nixpkgs / NixOS version to use.
  # inputs.nixpkgs.url = "nixpkgs/nixos-21.11";

  outputs = { self, nixpkgs }:
    let

      # System types to support.
      supportedSystems = [ "x86_64-linux" "x86_64-darwin" "aarch64-linux" "aarch64-darwin" ];

      # Helper function to generate an attrset '{ x86_64-linux = f "x86_64-linux"; ... }'.
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      # Nixpkgs instantiated for supported system types.
      nixpkgsFor = forAllSystems (system: import nixpkgs { inherit system; });

    in
    {

      # Provide some binary packages for selected system types.
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
        in
        {
          zpaste = with pkgs; rustPlatform.buildRustPackage rec {
            pname = "zpaste";
            version = "3.2.0";

            src = ./.;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkg-config ];

            buildInputs = [ sqlite zstd ];

            env.ZSTD_SYS_USE_PKG_CONFIG = true;

            meta = with lib; {
              description = "Zpaste is a pastebin based on wastebin";
              homepage = "https://github.com/Zakkaus/zpaste";
              license = licenses.mit;
              mainProgram = "zpaste";
            };
          };
        });

      defaultPackage = forAllSystems (system: self.packages.${system}.zpaste);
    };
}
