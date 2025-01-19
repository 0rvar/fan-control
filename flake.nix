{
  description = "Devshell with all the dependencies needed to develop and build the project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      # Boilerplate function for generating attributes for all systems
      forAllSystems =
        function:
        nixpkgs.lib.genAttrs
          [
            "x86_64-linux"
            "aarch64-linux"
            "x86_64-darwin"
            "aarch64-darwin"
          ]
          (
            system:
            (function (
              import nixpkgs {
                inherit system;
              }
            ))
              system
          );
    in
    {
      packages = forAllSystems (
        pkgs: system:
        let
          tools = with pkgs; [
            espflash
            ldproxy
          ];
          inputs = with pkgs; [
            libiconv
            SDL2
          ];
        in
        {
          default = pkgs.mkShell {
            buildInputs = tools ++ inputs;
            shellHook = ''
              export LIBCLANG_PATH="$HOME/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-18.1.2_20240912/esp-clang/lib"
              export PATH="$HOME/.rustup/toolchains/esp/xtensa-esp-elf/esp-14.2.0_20240906/xtensa-esp-elf/bin:$PATH"
            '';
          };
        }
      );
    };
}
