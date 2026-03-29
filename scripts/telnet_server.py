#!/usr/bin/env python3
"""
Telnet Shell Server - Provides a full shell terminal over Telnet

Architecture:
    Telnet Client <-> PTY Master <-> PTY Slave <-> shell process (zsh)

Usage:
    python3 scripts/telnet_server.py --port 2323
"""

import asyncio
import argparse
import logging
import os
import pty
import select
import struct
import fcntl
import termios
import signal
import sys
from typing import Optional


class ShellSession:
    """Manages a PTY-based shell session"""

    def __init__(self, shell_cmd: str, log_file: Optional[str]):
        self.shell_cmd = shell_cmd
        self.log_file = log_file
        self.master_fd = None
        self.slave_fd = None
        self.pid = None
        self.running = False

    def start(self):
        """Start the shell in a PTY"""
        self.master_fd, self.slave_fd = pty.openpty()

        # Set terminal size
        winsize = struct.pack("HHHH", 24, 80, 0, 0)
        try:
            fcntl.ioctl(self.master_fd, termios.TIOCSWINSZ, winsize)
        except:
            pass

        # Fork and exec the shell
        self.pid = os.fork()

        if self.pid == 0:  # Child process
            os.setsid()

            # Set the slave PTY as controlling terminal
            os.close(self.master_fd)
            os.dup2(self.slave_fd, 0)  # stdin
            os.dup2(self.slave_fd, 1)  # stdout
            os.dup2(self.slave_fd, 2)  # stderr
            os.close(self.slave_fd)

            # Execute the shell
            try:
                os.execvp(self.shell_cmd, [self.shell_cmd])
            except Exception as e:
                sys.stderr.write(f"Failed to exec shell: {e}\n")
                sys.stderr.flush()
                os._exit(1)
        else:  # Parent process
            os.close(self.slave_fd)
            self.running = True

    def stop(self):
        """Stop the shell session"""
        self.running = False

        if self.master_fd is not None:
            try:
                os.close(self.master_fd)
            except:
                pass
            self.master_fd = None

        if self.pid is not None:
            try:
                os.kill(self.pid, signal.SIGTERM)
                os.waitpid(self.pid, 0)
            except:
                pass
            self.pid = None

    def write_to_pty(self, data: bytes):
        """Write data to PTY"""
        if self.master_fd is not None:
            try:
                os.write(self.master_fd, data)
            except OSError:
                self.running = False

    def read_from_pty(self) -> bytes:
        """Read data from PTY (non-blocking)"""
        if self.master_fd is None:
            return b''

        try:
            ready, _, _ = select.select([self.master_fd], [], [], 0.01)
            if ready:
                data = os.read(self.master_fd, 4096)
                if not data:
                    self.running = False
                return data
        except OSError:
            self.running = False

        return b''


def filter_telnet_commands(data: bytes) -> bytes:
    """Filter out telnet IAC commands, return only the data"""
    result = bytearray()
    i = 0
    while i < len(data):
        if data[i] == 0xFF:  # IAC
            if i + 1 < len(data):
                cmd = data[i + 1]
                if cmd in (0xFB, 0xFC, 0xFD, 0xFE):  # WILL, WONT, DO, DONT
                    i += 3
                elif cmd == 0xFF:  # Escaped IAC
                    result.append(0xFF)
                    i += 2
                else:
                    i += 2
            else:
                i += 1
        else:
            result.append(data[i])
            i += 1
    return bytes(result)


async def handle_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter,
                       shell_cmd: str, log_file: Optional[str]):
    """Handle a single telnet client connection"""
    client_address = writer.get_extra_info('peername')
    logging.info(f"Client connected: {client_address}")

    # Send telnet options to enable character mode
    writer.write(b'\xFF\xFD\x03')  # IAC DO SUPPRESS_GO_AHEAD
    writer.write(b'\xFF\xFC\x01')  # IAC WONT ECHO
    await writer.drain()

    # Start shell session
    session = ShellSession(shell_cmd, log_file)
    session.start()

    try:
        loop = asyncio.get_event_loop()

        while session.running:
            # Check if client disconnected
            try:
                data = await asyncio.wait_for(reader.read(1024), timeout=0.05)
                if not data:
                    break

                # Filter telnet commands and write to PTY
                clean_data = filter_telnet_commands(data)
                if clean_data:
                    session.write_to_pty(clean_data)
            except asyncio.TimeoutError:
                pass
            except Exception:
                break

            # Read from PTY and send to client
            pty_data = session.read_from_pty()
            if pty_data:
                writer.write(pty_data)
                await writer.drain()

    except Exception as e:
        logging.error(f"Error handling client: {e}")
    finally:
        session.stop()
        writer.close()
        logging.info(f"Client disconnected: {client_address}")


async def start_server(host: str, port: int, shell_cmd: str, log_file: Optional[str]):
    """Start the telnet server"""
    server = await asyncio.start_server(
        lambda r, w: handle_client(r, w, shell_cmd, log_file),
        host,
        port
    )

    logging.info(f"Telnet shell server listening on {host}:{port}")
    logging.info(f"Shell: {shell_cmd}")
    logging.info("Press Ctrl+C to stop")

    async with server:
        await server.serve_forever()


def main():
    parser = argparse.ArgumentParser(description="Telnet Shell Server")
    parser.add_argument("--host", default="127.0.0.1", help="Host to bind to")
    parser.add_argument("--port", type=int, default=2323, help="Port to bind to")
    parser.add_argument("--shell", default="/bin/zsh", help="Shell command")
    parser.add_argument("--log", help="Log file path (optional)")

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(message)s',
        handlers=[logging.StreamHandler()]
    )

    try:
        asyncio.run(start_server(args.host, args.port, args.shell, args.log))
    except KeyboardInterrupt:
        logging.info("Server stopped")


if __name__ == "__main__":
    main()
