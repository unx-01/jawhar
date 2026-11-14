#!/usr/bin/env python3
import argparse
import os
import shutil
import signal
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OBJ = ROOT / "obj"

ARCHES = {
    "x86": {
        "rust_targets": ["i686-unknown-linux-gnu", "i686-linux-android"],
        "clang_target": "i386-unknown-none-elf",
        "ld_emulation": "elf_i386",
        "boot": "arch/x86/boot.S",
        "linker": "arch/x86/linker.ld",
        "qemu": ["qemu-system-i386", "qemu-system-x86_64"],
        "qemu_args": ["-m", "128M"],
    },
    "x86-64": {
        "rust_targets": ["x86_64-unknown-none", "x86_64-linux-android"],
        "clang_target": "x86_64-unknown-none-elf",
        "ld_emulation": "elf_x86_64",
        "boot": "arch/x86-64/boot.S",
        "linker": "arch/x86-64/linker.ld",
        "qemu": ["qemu-system-x86_64"],
        "qemu_args": ["-m", "128M"],
    },
    "aarch64": {
        "rust_targets": ["aarch64-unknown-none", "aarch64-linux-android"],
        "clang_target": "aarch64-unknown-none-elf",
        "ld_emulation": "aarch64elf",
        "boot": "arch/aarch64/boot.S",
        "linker": "arch/aarch64/linker.ld",
        "qemu": ["qemu-system-aarch64"],
        "qemu_args": ["-machine", "virt", "-cpu", "cortex-a57", "-m", "128M"],
    },
}


def find_tool(name, required=True):
    path = shutil.which(name)
    if path is None and required:
        sys.exit(f"error: required tool not found: {name}")
    return path


def run(cmd):
    cmd = [str(part) for part in cmd]
    print("+", " ".join(cmd))
    subprocess.run(cmd, cwd=ROOT, check=True)


def installed_rust_targets(rustc):
    sysroot = subprocess.check_output([rustc, "--print", "sysroot"], text=True).strip()
    libdir = Path(sysroot) / "lib" / "rustlib"
    if not libdir.is_dir():
        return set()
    return {entry.name for entry in libdir.iterdir() if entry.is_dir()}


def choose_rust_target(arch, cfg, installed):
    for target in cfg["rust_targets"]:
        if target in installed:
            return target
    wanted = " or ".join(cfg["rust_targets"])
    sys.exit(f"error: no Rust core library for {arch}; install one of: {wanted}")


def output_paths(arch):
    out = OBJ / arch
    return out, out / "boot.o", out / "init.o", out / "jawhar.elf"


def build(arch):
    cfg = ARCHES[arch]
    rustc = find_tool("rustc")
    clang = find_tool("clang")
    ld = find_tool("ld.lld")
    target = choose_rust_target(arch, cfg, installed_rust_targets(rustc))

    out, boot, init, elf = output_paths(arch)
    out.mkdir(parents=True, exist_ok=True)

    run([
        rustc,
        "--edition", "2021",
        "-O",
        "--target", target,
        "--emit=obj",
        "-C", "panic=abort",
        "-C", "relocation-model=static",
        "-C", "debuginfo=0",
        "--crate-name", "jawhar",
        "arch/init.rs",
        "-o", init,
    ])
    run([
        clang,
        f"--target={cfg['clang_target']}",
        "-c",
        cfg["boot"],
        "-o", boot,
    ])
    run([
        ld,
        "-m", cfg["ld_emulation"],
        "-T", cfg["linker"],
        "--gc-sections",
        "-static",
        "-nostdlib",
        "-o", elf,
        boot,
        init,
    ])
    print(f"built {elf.relative_to(ROOT)}")


def stop(arch):
    pidfile = OBJ / arch / "qemu.pid"
    if not pidfile.exists():
        return
    pid = int(pidfile.read_text().strip())
    try:
        os.kill(pid, signal.SIGTERM)
        print(f"stopped QEMU for {arch} (pid {pid})")
    except ProcessLookupError:
        print(f"QEMU for {arch} is not running")
    pidfile.unlink(missing_ok=True)


def run_kernel(arch, vnc=None, daemonize=True):
    cfg = ARCHES[arch]
    qemu = None
    for name in cfg["qemu"]:
        qemu = find_tool(name, required=False)
        if qemu:
            break
    if qemu is None:
        sys.exit(f"error: no QEMU binary found for {arch}")

    build(arch)
    out, _, _, elf = output_paths(arch)
    kernel = elf
    if arch == "x86-64":
        objcopy = find_tool("llvm-objcopy", required=False) or find_tool("objcopy")
        kernel = elf.with_name("jawhar32.elf")
        run([objcopy, "-O", "elf32-i386", elf, kernel])

    args = [
        qemu,
        *cfg["qemu_args"],
        "-kernel", kernel,
        "-monitor", "none",
        "-no-reboot",
    ]

    if vnc is None:
        args += ["-display", "none", "-serial", "stdio"]
    else:
        stop(arch)
        pidfile = out / "qemu.pid"
        args += [
            "-vnc", vnc,
            "-serial", f"file:{out / 'serial.log'}",
            "-pidfile", pidfile,
        ]
        if daemonize:
            args.append("-daemonize")

    run(args)

    if vnc is not None and daemonize:
        print(f"VNC server running on {vnc}")


def clean():
    for arch in ARCHES:
        stop(arch)
        out, boot, init, elf = output_paths(arch)
        for path in (boot, init, elf, elf.with_name("jawhar32.elf"), out / "serial.log"):
            if path.exists():
                path.unlink()
                print(f"removed {path.relative_to(ROOT)}")
        if out.is_dir() and not any(out.iterdir()):
            out.rmdir()


def main():
    parser = argparse.ArgumentParser(description="Build and run the jawhar kernel")
    parser.add_argument("command", nargs="?", default="build", choices=["build", "run", "stop", "clean"])
    parser.add_argument("arch", nargs="*", choices=[*ARCHES, "all"], default=None)
    parser.add_argument("--vnc", nargs="?", const=":0", default=None, help="run QEMU with a VNC display")
    parser.add_argument("--no-daemon", action="store_true", help="keep QEMU in the foreground with VNC")
    args = parser.parse_args()

    if args.command == "clean":
        clean()
        return

    if args.command == "stop":
        arches = args.arch or list(ARCHES)
        if "all" in arches:
            arches = list(ARCHES)
        for arch in arches:
            stop(arch)
        return

    arches = args.arch or (["x86-64"] if args.command == "run" else list(ARCHES))
    if "all" in arches:
        arches = list(ARCHES)

    for arch in arches:
        if args.command == "build":
            build(arch)
        else:
            run_kernel(arch, args.vnc, not args.no_daemon)


if __name__ == "__main__":
    main()
