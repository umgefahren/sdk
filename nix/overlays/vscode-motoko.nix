self: super:

let src = builtins.fetchGit {
  url = "ssh://git@github.com/dfinity-lab/vscode-motoko";
  ref = "nix-from-scratch";
  rev = "53a65af37a0b369f324f71eb96914a597b04c4c3";
}; in
{
  vscode-motoko = import src { pkgs = self.pkgs; };
}

