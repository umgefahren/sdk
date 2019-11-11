self: super:

let src = builtins.fetchGit {
  url = "ssh://git@github.com/dfinity-lab/vscode-motoko";
  ref = "master";
  rev = "dec6f40bab5b0321be1b58c0c0d1808b91f1c835";
}; in
{
  vscode-motoko = import src { pkgs = self.pkgs; };
}

