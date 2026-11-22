#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
═══════════════════════════════════════════════════════════════════════════
🎙️ XVoice Launcher — ระบบไมค์ตามระยะ Minecraft Bedrock (All-in-One)
═══════════════════════════════════════════════════════════════════════════
สคริปต์ครบจบในไฟล์เดียวสำหรับรันบอทเสียง XVoice และ Cloudflare Tunnel
ไม่ต้องติดตั้งไลบรารี Python เพิ่ม (ใช้ Standard Library 100%)
"""

import os
import sys
import glob
import json
import time
import shutil
import string
import secrets
import subprocess
import urllib.request
import urllib.error

if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
EXE_NAME = "zircon-voice.exe" if sys.platform == "win32" else "zircon-voice-linux"
EXE_PATH = os.path.join(BASE_DIR, EXE_NAME)
CONFIG_ENV = os.path.join(BASE_DIR, "config.env")
CONFIG_EXAMPLE = os.path.join(BASE_DIR, "config.env.example")
DEFAULT_SECRET = "mcpe-voice-secret-zirconx"


def clear_screen():
    os.system("cls" if os.name == "nt" else "clear")


def get_wt_executable():
    """ค้นหา Windows Terminal (wt.exe) ถ้ามี"""
    if sys.platform != "win32":
        return None
    alias = os.path.expandvars(r"%LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe")
    if os.path.exists(alias):
        return alias
    for pkg_wt in glob.glob(r"C:\Program Files\WindowsApps\Microsoft.WindowsTerminal*\wt.exe"):
        if os.path.getsize(pkg_wt) > 0:
            return pkg_wt
    return shutil.which("wt.exe") or shutil.which("wt")


def launch_self_in_new_window(title, flag):
    """เปิดตัวเองในหน้าต่างใหม่ด้วยโหมดที่กำหนด (--voice หรือ --tunnel)"""
    script_path = os.path.abspath(__file__)
    python_exe = sys.executable

    if sys.platform == "win32":
        wt_exe = get_wt_executable()
        if wt_exe:
            try:
                cmd = f'"{wt_exe}" --title "{title}" -d "{BASE_DIR}" "{python_exe}" "{script_path}" {flag}'
                subprocess.Popen(cmd, shell=True, close_fds=True)
                return True
            except Exception:
                pass
        # Fallback: เปิดด้วย CMD แยกหน้าต่าง
        try:
            subprocess.Popen(
                ["cmd", "/c", "start", title, python_exe, script_path, flag],
                cwd=BASE_DIR,
                close_fds=True,
            )
            return True
        except Exception as e:
            print(f"[!] ไม่สามารถเปิดหน้าต่างใหม่ได้: {e}")
            return False
    else:
        subprocess.Popen([python_exe, script_path, flag], cwd=BASE_DIR, close_fds=True)
        return True


def generate_secret(length=10):
    """สุ่มรหัสลับที่ปลอดภัยและอ่านง่าย"""
    safe_chars = "".join(c for c in (string.ascii_lowercase + string.digits) if c not in "o0l1")
    return "xv-" + "".join(secrets.choice(safe_chars) for _ in range(length))


def ensure_config_and_secret():
    """ตรวจสอบ config.env และสร้าง API_SECRET ให้อัตโนมัติถ้ายังเป็นค่าเริ่มต้น"""
    if not os.path.exists(CONFIG_ENV):
        if os.path.exists(CONFIG_EXAMPLE):
            shutil.copy2(CONFIG_EXAMPLE, CONFIG_ENV)
            print("[!] สร้างไฟล์ config.env จากต้นแบบแล้ว")
            print("    กรุณาเปิดไฟล์ config.env แล้วใส่ DISCORD_TOKEN และ GUILD_ID ให้เรียบร้อยก่อนใช้งาน")
            return None, True
        else:
            print("[!] ไม่พบไฟล์ config.env และ config.env.example")
            return None, True

    try:
        with open(CONFIG_ENV, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except Exception as e:
        print(f"[!] อ่านไฟล์ config.env ไม่สำเร็จ: {e}")
        return None, True

    secret_val = None
    updated = False
    new_lines = []

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("API_SECRET="):
            val = stripped.split("=", 1)[1].strip()
            if not val or val == DEFAULT_SECRET:
                secret_val = generate_secret()
                new_lines.append(f"API_SECRET={secret_val}\n")
                updated = True
            else:
                secret_val = val
                new_lines.append(line)
        else:
            new_lines.append(line)

    if secret_val is None:
        secret_val = generate_secret()
        if new_lines and not new_lines[-1].endswith("\n"):
            new_lines[-1] += "\n"
        new_lines.append(f"API_SECRET={secret_val}\n")
        updated = True

    if updated:
        try:
            with open(CONFIG_ENV, "w", encoding="utf-8") as f:
                f.writelines(new_lines)
            print(f"[*] สุ่มรหัส API_SECRET ใหม่และบันทึกลง config.env แล้ว: {secret_val}")
        except Exception:
            pass

    return secret_val, False


def cleanup_old_bot():
    """ปิดโพรเซส zircon-voice เก่าที่อาจค้างอยู่"""
    if sys.platform == "win32":
        subprocess.run(
            ["taskkill", "/F", "/IM", "zircon-voice.exe"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )


def run_voice_bot():
    """รันบอทเสียง XVoice"""
    cleanup_old_bot()
    print("╭──────────────────────────────────────────────────────────────────╮")
    print("│  🎙️  XVoice Discord Bot — Proximity VoiceChat                    │")
    print("╰──────────────────────────────────────────────────────────────────╯\n")

    secret, need_setup = ensure_config_and_secret()
    if need_setup:
        input("\n👉 กด Enter เพื่อปิดหน้าต่าง...")
        return

    if not os.path.exists(EXE_PATH):
        print(f"[!] ไม่พบไฟล์บอท: {EXE_PATH}")
        input("\n👉 กด Enter เพื่อปิดหน้าต่าง...")
        return

    print(f"[*] API_SECRET ของคุณ: {secret}")
    print("    (นำรหัสนี้ไปใส่ในเมนูแอดมินในเกมผ่านไอเทมรีโมทดิสคอร์ด)")
    print(f"[*] กำลังเริ่มทำงาน {EXE_NAME} ... (กด Ctrl+C เพื่อหยุด)\n")
    print("═" * 68)

    try:
        subprocess.run([EXE_PATH], cwd=BASE_DIR)
    except KeyboardInterrupt:
        print("\n[✓] ปิดการทำงานบอทเสียงเรียบร้อยแล้ว")
    except Exception as e:
        print(f"\n[!] เกิดข้อผิดพลาด: {e}")

    print("═" * 68)
    input("\n👉 กด Enter เพื่อปิดหน้าต่าง...")


def find_cloudflared():
    """ค้นหา cloudflared ในโฟลเดอร์โปรเจกต์หรือ PATH ของระบบ"""
    exe = "cloudflared.exe" if sys.platform == "win32" else "cloudflared"
    candidates = [
        os.path.join(BASE_DIR, exe),
        os.path.join(BASE_DIR, "bin", exe),
        os.path.join(os.path.dirname(BASE_DIR), "bin", exe),
    ]
    for path in candidates:
        if os.path.exists(path):
            return path
    return shutil.which("cloudflared") or shutil.which("cloudflared.exe")


def run_tunnel():
    """รัน Cloudflare Quick Tunnel ชี้มาที่พอร์ต 3000"""
    print("╭──────────────────────────────────────────────────────────────────╮")
    print("│  🚇  Cloudflare Quick Tunnel — XVoice                            │")
    print("╰──────────────────────────────────────────────────────────────────╯")
    print("💡 ปลายทาง: http://127.0.0.1:3000")
    print("   เมื่อขึ้นลิงก์ https://xxxx-xxxx.trycloudflare.com")
    print("   ให้นำไปใส่ช่อง API URL ในเมนูแอดมินของเกม\n")

    cf_exe = find_cloudflared()
    if not cf_exe:
        print("[!] ไม่พบโปรแกรม cloudflared ในเครื่องนี้")
        print("    วิธีติดตั้งง่ายๆ (รันใน PowerShell):")
        print("      winget install Cloudflare.cloudflared")
        print("    หรือดาวน์โหลดไฟล์ cloudflared.exe มาวางไว้ในโฟลเดอร์เดียวกับ run.py นี้")
        input("\n👉 กด Enter เพื่อปิดหน้าต่าง...")
        return

    print(f"[*] ใช้โปรแกรม: {cf_exe}")
    print("[*] กำลังสร้าง Tunnel ... (กด Ctrl+C เพื่อหยุด)\n")
    print("═" * 68)

    try:
        subprocess.run([cf_exe, "tunnel", "--url", "http://127.0.0.1:3000"], cwd=BASE_DIR)
    except KeyboardInterrupt:
        print("\n[✓] ปิด Cloudflare Tunnel เรียบร้อยแล้ว")
    except Exception as e:
        print(f"\n[!] เกิดข้อผิดพลาด: {e}")

    print("═" * 68)
    input("\n👉 กด Enter เพื่อปิดหน้าต่าง...")


def check_health():
    """ตรวจสอบสถานะการทำงานของบอทเสียงที่พอร์ต 3000"""
    print("\n[*] กำลังตรวจสอบสถานะบอทที่ http://127.0.0.1:3000/api/health ...")
    try:
        with urllib.request.urlopen("http://127.0.0.1:3000/api/health", timeout=3) as resp:
            data = json.loads(resp.read().decode("utf-8", errors="replace"))
            status = data.get("status", "unknown")
            bot = data.get("bot", "unknown")
            rooms = data.get("rooms", 0)
            tracked = data.get("tracked", 0)
            print(f"[✓] บอทออนไลน์ปกติ! (API: {status} | Discord: {bot})")
            print(f"    ห้องเสียงที่เปิดอยู่: {rooms} | ผู้เล่นที่เชื่อมต่อ: {tracked}")
    except urllib.error.URLError:
        print("[!] ติดต่อบอทไม่ได้ — กรุณากด [1] เพื่อเปิดบอทเสียงก่อน")
    except Exception as e:
        print(f"[!] ตรวจสอบไม่สำเร็จ: {e}")
    input("\n👉 กด Enter เพื่อกลับสู่เมนูหลัก...")


def print_menu():
    clear_screen()
    print("╭──────────────────────────────────────────────────────────────────╮")
    print("│  🎙️  XVoice Launcher — ระบบไมค์ตามระยะ Minecraft Bedrock         │")
    print("╰──────────────────────────────────────────────────────────────────╯")
    print()
    print("   [1] 🎙️  เปิดบอทเสียง XVoice         (เปิดหน้าต่างใหม่)")
    print("   [2] 🚇  เปิด Cloudflare Tunnel       (สำหรับเชื่อมเซิร์ฟนอก)")
    print("   [3] 🚀  เปิดทั้งคู่พร้อมกัน           (บอทเสียง + Tunnel)")
    print("   [4] 🩺  เช็กสถานะบอทเสียง            (http://127.0.0.1:3000)")
    print()
    print("   [0] ❌  ออกจากโปรแกรม")
    print("────────────────────────────────────────────────────────────────────")


def main():
    # โหมดรันตรงเมื่อถูกเรียกจากหน้าต่างใหม่
    if len(sys.argv) > 1:
        arg = sys.argv[1].lower().strip("-")
        if arg in ["voice", "bot"]:
            run_voice_bot()
            return
        if arg in ["tunnel", "cf"]:
            run_tunnel()
            return
        if arg == "health":
            check_health()
            return

    while True:
        print_menu()
        try:
            choice = input("👉 เลือกการทำงาน [0-4]: ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\n\nขอบคุณที่ใช้งาน XVoice ครับ!")
            break

        if choice == "1":
            print("\n[*] กำลังเปิดบอทเสียง XVoice...")
            if launch_self_in_new_window("🎙️ XVoice Bot", "--voice"):
                print("[✓] เปิดหน้าต่างบอทเสียงแล้ว")
            input("\n👉 กด Enter เพื่อกลับสู่เมนูหลัก...")
        elif choice == "2":
            print("\n[*] กำลังเปิด Cloudflare Tunnel...")
            if launch_self_in_new_window("🚇 Cloudflare Tunnel", "--tunnel"):
                print("[✓] เปิดหน้าต่าง Tunnel แล้ว")
            input("\n👉 กด Enter เพื่อกลับสู่เมนูหลัก...")
        elif choice == "3":
            print("\n[*] กำลังเปิดบอทเสียงและ Cloudflare Tunnel พร้อมกัน...")
            launch_self_in_new_window("🎙️ XVoice Bot", "--voice")
            time.sleep(1.0)
            launch_self_in_new_window("🚇 Cloudflare Tunnel", "--tunnel")
            print("[✓] เปิดทั้ง 2 หน้าต่างเรียบร้อยแล้ว")
            input("\n👉 กด Enter เพื่อกลับสู่เมนูหลัก...")
        elif choice == "4":
            check_health()
        elif choice in ["0", "q", "quit", "exit"]:
            print("\nขอบคุณที่ใช้งาน XVoice ครับ!")
            break
        else:
            input("\n[!] ตัวเลือกไม่ถูกต้อง (กรุณาใส่ตัวเลข 0-4) กด Enter เพื่อลองใหม่...")


if __name__ == "__main__":
    main()
