# git-aside

Version sensitive files (CLAUDE.md, .claude/, .env, secrets...) in a **separate private repo**, **transparently** alongside your usual git commands.

## The problem

```
my-project/           <- public repo
├── .gitignore        <- ignores CLAUDE.md, .claude/
├── CLAUDE.md         <- you want this versioned ELSEWHERE
├── .claude/          <- same
└── src/
```

You want CLAUDE.md and .claude/ versioned in a private repo, without changing your git workflow at all.

## Installation

### Prerequisites
- Rust + Cargo: https://rustup.rs
- Git

### Build & install

```bash
git clone https://github.com/JSpatim/git-aside
cd git-aside
cargo install --path .
```

The `git-aside` binary is installed in `~/.cargo/bin/` (already in your PATH if Rust is installed).

Git automatically recognizes `git aside` as a subcommand (`git-<name>` convention).

## Usage

### Setup (once per project)

```bash
cd my-project

git aside init git@github.com:you/my-project-private.git CLAUDE.md .claude/
```

What this does:
- Creates a bare repo in `~/.git-asides/<id>/repo.git`
- Adds `CLAUDE.md` and `.claude/` to the main repo's `.gitignore`
- Installs git hooks (pre-commit, pre-push, post-merge, post-checkout)
- Makes an initial commit + push if the files already exist

### Daily workflow — nothing changes

```bash
# Your usual commands work as before
git add src/
git commit -m "feat: new feature"   # -> also commits the aside if modified
git push                             # -> also pushes the aside
git pull                             # -> also pulls the aside
```

### Manual commands (if needed)

```bash
git aside status          # show aside repo status
git aside sync            # manual add + commit + push
git aside push            # push only
git aside pull            # pull only
git aside add .env        # add a new file to the aside
git aside deinit          # remove git-aside from this project
```

## On a new machine

```bash
# 1. Clone your main repo
git clone git@github.com:JSpatim/my-project.git
cd my-project

# 2. Re-initialize git-aside for this project
git aside init git@github.com:JSpatim/my-project-private.git CLAUDE.md .claude/
```

## Architecture

```
~/.git-asides/
└── <project-id>/
    ├── repo.git/          <- aside bare repo
    └── config.toml        <- remote, work-tree, tracked files
```

The config is **never** stored in the main repo — only in `~/.git-asides/`.

## Files installed in the main repo

Only `.gitignore` is modified (tracked file entries are added).
Git hooks are added to `.git/hooks/` (not versioned).

## License

[MIT](LICENSE)
