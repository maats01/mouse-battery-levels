#!/usr/bin/env python3
"""Throwaway spike: send one COMPX v1 command and dump the reply.

usage: probe.py /dev/hidrawN CMD_HEX [repeat]
"""
import os, select, sys, time

REPORT_ID = 0x08


def frame(cmd):
    p = bytearray(16)
    p[0] = cmd
    p[15] = (0x55 - ((REPORT_ID + sum(p[:15])) & 0xFF)) & 0xFF
    return bytes([REPORT_ID]) + bytes(p)


def main():
    dev, cmd = sys.argv[1], int(sys.argv[2], 16)
    fd = os.open(dev, os.O_RDWR | os.O_NONBLOCK)
    try:
        req = frame(cmd)
        t0 = time.monotonic()
        os.write(fd, req)
        print(">>", req.hex(" "))
        deadline = t0 + 0.3
        while True:
            left = deadline - time.monotonic()
            if left <= 0:
                print(f"timeout ({(time.monotonic()-t0)*1000:.1f} ms)")
                break
            r, _, _ = select.select([fd], [], [], left)
            if not r:
                continue
            data = os.read(fd, 65)
            ms = (time.monotonic() - t0) * 1000
            print(f"<< {data.hex(' ')}  ({ms:.1f} ms)")
            if len(data) >= 2 and data[0] == REPORT_ID and data[1] == cmd:
                break
    finally:
        os.close(fd)


if __name__ == "__main__":
    main()
