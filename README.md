# Bachelorarbeit — Erweiterung der Rust-Standardbibliothek fur das Betriebssystem D3OS

Abgabe-Repository von Mate-Trpmir Jakulj, Institut für Informatik, Heinrich-Heine-Universität Düsseldorf.

Dieses Repository enthält die schriftliche Ausarbeitung und den vollständigen Quellcode
beider Teilprojekte samt Entwicklungsgeschichte.

## Aufbau

| Pfad | Inhalt |
| --- | --- |
| `document/` | Schriftliche Ausarbeitung |
| `project/D3OS/` | D3OS mit den Anpassungen für den std-Port (Fork von `hhu-bsinfo/D3OS`) |
| `project/rust/` | Rust-Fork mit dem std-Backend und dem Target-Spec (Fork von `rust-lang/rust`) |
| `Aufgabenstellung_-_Jakulj.pdf` | Aufgabenstellung |

Die beiden Projektordner wurden per `git subtree add` **ohne** `--squash` eingebunden.
Dadurch ist die vollständige Commit-Historie beider Forks in diesem Repository enthalten
und die ursprünglichen Commit-IDs sind erhalten geblieben.

## Eigene Commits

31 Commits, verteilt auf drei Branches:

| Ort | Anzahl | Inhalt |
| --- | --- | --- |
| `master`, unter `project/D3OS/` | 14 | Kernel, Bibliotheken, Testanwendungen |
| `master`, unter `project/rust/` | 10 | std-Backends, Target-Spec, Toolchain-Upgrades |
| Branch `rust-2025-08-18` | 3 | Vorbereitende Fixes für das erste Toolchain-Upgrade |
| Branch `rust-2025-10-20` | 4 | Dieselben Fixes, erneut aufgesetzt, plus ein Upgrade |

Die beiden Zusatz-Branches enthalten Arbeitsstände aus früheren Toolchain-Upgrades. Sie
wurden bei jedem Upgrade neu auf den frischen Upstream-Stand aufgesetzt und tragen deshalb
eigene Commit-IDs, obwohl sie inhaltlich dieselbe Änderung enthalten.

Alle eigenen Commits auflisten:

```bash
git log --all --author=m4te855 --no-merges --oneline
```

Eine vollständige Aufstellung mit Dateien und Commit-Nachrichten liegt der Ausarbeitung bei.

## Bauen und Ausführen

Getestet unter Ubuntu 24.04.

### 1. Systempakete

```bash
sudo apt install rustup build-essential nasm dosfstools wget qemu-system-x86 ninja-build cmake python3
cargo install --no-default-features cargo-make cargo-license
```

### 2. Rust-Toolchain bauen

D3OS lässt sich nicht mit einer regulären Rust-Toolchain übersetzen: `rust-toolchain.toml`
verlangt die Toolchain `dev-x86_64-unknown-d3os`, die das Ziel `x86_64-unknown-d3os` samt
portierter Standardbibliothek mitbringt. Sie wird aus `project/rust/` gebaut.

```bash
cd project/rust
./x build library --target x86_64-unknown-d3os
rustup toolchain link dev-x86_64-unknown-d3os build/host/stage1
```

Der erste Durchlauf dauert lange: `bootstrap.toml` setzt `download-ci-llvm = false`, LLVM
wird also aus dem Quelltext übersetzt. Je nach Rechner sind mehrere Stunden einzuplanen.

Prüfen, ob die Toolchain registriert ist:

```bash
rustup toolchain list
```

### 3. D3OS bauen und starten

```bash
cd project/D3OS
cargo make --no-workspace
```

## Testanwendungen des std-Ports

Im laufenden System stehen in der Shell vier Anwendungen bereit, die je ein Backend der
portierten Standardbibliothek prüfen. Jede entspricht genau einer Datei aus dem Port, ein
Fehlschlag zeigt damit direkt auf das zuständige Backend.

| Befehl | Prüft | Backend |
| --- | --- | --- |
| `stdiotest` | `print!`, `println!`, `stdout().lock().write`, `read`, `read_line` | `sys/stdio/d3os.rs` |
| `stdheaptest` | Große Allokation, Alloc/Free-Zyklen, Über-Alignment, Vec-Wachstum | `sys/pal/d3os/alloc.rs` |
| `stdfstest` | Round-Trip, Seek-Varianten, `create_new`, `std::fs`-Helfer | `sys/fs/d3os.rs` |
| `stdthreadtest` | `spawn`/`join`, `Arc`-Zähler, `yield_now`, `sleep` | `sys/thread/d3os.rs` |

`stdiotest` erwartet Eingaben: erst eine Zeile für den `read_line`-Test, danach beliebig
viele Zeilen für die interaktive Schleife, die eine leere Zeile beendet.
