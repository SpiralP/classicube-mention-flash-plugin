{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      makePackages = (pkgs:
        let
          rustManifest = lib.importTOML ./Cargo.toml;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = rustManifest.package.name;
            version = rustManifest.package.version;

            src = lib.sourceByRegex ./. [
              "^\.cargo(/.*)?$"
              "^build\.rs$"
              "^Cargo\.(lock|toml)$"
              "^src(/.*)?$"
            ];

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
                "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-V5PBZR0rj42crA1fGUjMk4rDh0ZpjjNcbMCe6bgotW8=";
              };
            };

            buildInputs = with pkgs; [
              xorg.libX11
            ];
            nativeBuildInputs = with pkgs; [
              pkg-config
              rustPlatform.bindgenHook
            ];
          };
        });
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };

          packages = makePackages pkgs;
        in
        {
          devShells.${system} = packages // {
            default =
              let
                allDrvsIn = (name:
                  lib.lists.flatten (
                    builtins.map
                      (drv: drv.${name} or [ ])
                      (builtins.attrValues packages)
                  ));
              in
              pkgs.mkShell {
                name = "dev-shell";
                packages = with pkgs; [
                  clippy
                  rustfmt
                  rust-analyzer
                ];
                buildInputs = allDrvsIn "buildInputs";
                nativeBuildInputs = allDrvsIn "nativeBuildInputs";
                propagatedBuildInputs = allDrvsIn "propagatedBuildInputs";
                propagatedNativeBuildInputs = allDrvsIn "propagatedNativeBuildInputs";
              };
          };
          packages.${system} = packages // {
            default = pkgs.linkFarmFromDrvs "link-farm" (builtins.attrValues packages);
          };
        })
      lib.systems.flakeExposed);
}


