{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc
    cargo
    clippy
    rustfmt
    pre-commit
    git
    direnv
    nix-direnv
  ];

  shellHook = ''
    if [ ! -f .git/hooks/pre-commit ]; then
      echo "Installing pre-commit hooks..."
      pre-commit install
      echo "Pre-commit hooks installed"
    fi
  '';
}