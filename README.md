# Bachelorarbeit — Erweiterung der Rust-Standardbibliothek für das Betriebssystem D3OS

Öffentliche Fassung des Abgabe-Repositorys von Mate-Trpimir Jakulj, Institut für Informatik, Heinrich-Heine-Universität Düsseldorf.

Dieses Repository enthält die schriftliche Ausarbeitung und den vollständigen Quellcode
beider Teilprojekte samt Entwicklungsgeschichte.

## Aufbau

| Pfad | Inhalt |
| --- | --- |
| `Bachelorarbeit.pdf` | Schriftliche Ausarbeitung |
| `project/D3OS/` | D3OS mit den Anpassungen für den std-Port (Fork von `hhu-bsinfo/D3OS`) |
| `project/rust/` | Rust-Fork mit dem std-Backend und dem Target-Spec (Fork von `rust-lang/rust`) |
| `.gitmodules` | Zuordnung der Submodule des Rust-Baums auf ihre Pfade unter `project/rust/` |

Die beiden Projektordner wurden per `git subtree add` **ohne** `--squash` eingebunden.
Dadurch ist die vollständige Commit-Historie beider Forks in diesem Repository enthalten
und die ursprünglichen Commit-IDs sind erhalten geblieben.

## Eigene Commits

Die eigenen Commits liegen auf `master` sowie auf zwei Zusatz-Branches:

| Ort | Inhalt |
| --- | --- |
| `master`, unter `project/D3OS/` | Kernel, Bibliotheken, Testanwendungen |
| `master`, unter `project/rust/` | std-Backends, Target-Spec, Toolchain-Upgrades |
| Branch `rust-2025-08-18` | Vorbereitende Fixes für das erste Toolchain-Upgrade |
| Branch `rust-2025-10-20` | Dieselben Fixes, erneut aufgesetzt, plus ein Upgrade |

Die beiden Zusatz-Branches enthalten Arbeitsstände aus früheren Toolchain-Upgrades. Sie
wurden bei jedem Upgrade neu auf den frischen Upstream-Stand aufgesetzt und tragen deshalb
eigene Commit-IDs, obwohl sie inhaltlich dieselbe Änderung enthalten.

Die Commits auf `master` sind in Anhang A der Ausarbeitung (`Bachelorarbeit.pdf`)
vollständig aufgeführt, mit Hash, Datum, Projektteil und Commit-Nachricht. Wer einen davon
im Repository ansehen möchte, hängt `/commit/<Hash>` an die Adresse dieses Repositorys an.

## Hinweis zur öffentlichen Fassung

Gegenüber dem Abgabe-Repository fehlen die Aufgabenstellung, die LaTeX-Quellen der
Ausarbeitung und persönliche Angaben. Sie wurden mit `git filter-repo` aus der gesamten
Historie entfernt. Die Commits aus den beiden Forks behalten dabei ihre ursprünglichen IDs.
Neue IDs haben nur die Commits, die direkt in diesem Repository entstanden sind. Von den
Commits aus Anhang A betrifft das nur `95feda3`, der hier `9caeefb` heißt.

## Bauen und Ausführen

Getestet unter Ubuntu 24.04. Die Schritte 3 bis 5 laufen nacheinander in derselben Shell
im Verzeichnis `project/rust`, da Schritt 5 die in Schritt 4 gesetzte Variable
`RUST_STAGE1` weiterverwendet.

Gebaut wird direkt in `project/`. Eine Eigenheit des Rust-Forks ist dabei zu beachten.
Seine Submodule, darunter `src/llvm-project` und `src/tools/cargo`, sind wie in jedem
Rust-Checkout keine Dateien, sondern Verweise. Die Zuordnung dazu steht in
`project/rust/.gitmodules` mit Pfaden ohne das Präfix `project/rust/` und greift aus dem
Subtree heraus nicht. Dieses Repository enthält deshalb im Wurzelverzeichnis eine eigene
`.gitmodules`, in der dieselben zwölf Einträge mit vorangestelltem Pfad stehen. Der
Aufruf in Schritt 2 löst die Submodule darüber auf.

### 1. Systempakete

```bash
sudo apt install rustup build-essential nasm dosfstools wget qemu-system-x86 ninja-build cmake python3 pkg-config libssl-dev
rustup default stable
cargo install --no-default-features cargo-make cargo-license
```

`rustup default stable` muss zwischen beiden Befehlen stehen. Eine frisch eingerichtete
rustup-Umgebung kennt noch keine Standard-Toolchain und bricht sonst mit
„rustup could not choose a version of cargo to run" ab.

### 2. Repository klonen

```bash
git clone https://github.com/m4te855/bachelorarbeit-d3os.git abgabe
cd abgabe
git submodule update --init --recursive
```

Der zweite Aufruf muss im Wurzelverzeichnis erfolgen und nicht in `project/rust`, da nur
die `.gitmodules` der Wurzel die vollständigen Pfade enthält. Der Klon umfasst rund
750 MB, die Submodule des Rust-Baums noch einmal etwa 5,5 GB.

### 3. Rust-Toolchain bauen

D3OS lässt sich nicht mit einer regulären Rust-Toolchain übersetzen. `rust-toolchain.toml`
verlangt die Toolchain `dev-x86_64-unknown-d3os`, die das Ziel `x86_64-unknown-d3os` samt
portierter Standardbibliothek mitbringt.

Beide Ziele stehen in `bootstrap.toml` unter `[build] target`, weshalb `./x build` ohne
`--target` genügt.

```bash
cd project/rust
./x build
```

Der Lauf beginnt mit der Warnung

```
warn: Looks like you are trying to bootstrap Rust from a source that is neither a git
clone nor distributed tarball.
This build may fail due to missing submodules unless you put them in place manually.
```

Sie ist folgenlos, sofern Schritt 2 vollständig durchgelaufen ist. Das Bootstrap-Skript
prüft in `src/bootstrap/bootstrap.py`, ob im Wurzelverzeichnis des Rust-Baums ein
Verzeichnis `.git` liegt. Als Subtree liegt dort keines, denn das Git-Verzeichnis gehört
dem umgebenden Repository. Das Skript hält den Baum deshalb weder für einen Klon noch für
ein Quelltextarchiv und verwaltet die Submodule nicht selbst. Genau darum werden sie in
Schritt 2 von Hand geholt. Wird das versäumt, bricht der Bau später mit
„submodule library/backtrace does not appear to be checked out" ab.

Der erste Durchlauf dauert lange, denn `bootstrap.toml` setzt `download-ci-llvm = false`
und LLVM wird damit aus dem Quelltext übersetzt. Je nach Rechner sind ein bis zwei Stunden
und einige zehn Gigabyte Plattenplatz einzuplanen. Die Zeile `jobs = 4` begrenzt die
Parallelität, da der Bau sonst den Hauptspeicher überschreiten kann.

### 4. Toolchain registrieren

`rustup toolchain link` legt weder eine Kopie noch einen Symlink an, sondern schreibt
lediglich einen Pfadeintrag. Ein bereits vorhandener Eintrag wird deshalb zuerst entfernt,
damit kein alter Pfad stehen bleibt.

```bash
RUST_STAGE1=$(realpath build/x86_64-unknown-linux-gnu/stage1)
rustup toolchain uninstall dev-x86_64-unknown-d3os 2>/dev/null
rustup toolchain link dev-x86_64-unknown-d3os "$RUST_STAGE1"
rustup toolchain list
```

### 5. Cargo nachlegen

Der Lauf aus Schritt 3 erzeugt den Compiler samt Standardbibliothek, legt aber kein
`cargo` in das Stage-1-Verzeichnis. Da die verlinkte Toolchain in Schritt 6 ein eigenes
`cargo` benötigt, wird es in einem zweiten Lauf gebaut und daneben kopiert.

```bash
./x build cargo
CARGO=$(find build -type f -name cargo -path '*stage1-tools-bin/*' | head -1)
cp "$CARGO" "$RUST_STAGE1/bin/cargo" && chmod +x "$RUST_STAGE1/bin/cargo"
```

### 6. D3OS bauen und starten

```bash
cd ../D3OS
cargo make clean
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
