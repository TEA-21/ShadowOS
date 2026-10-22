import type { VsockLogEntry } from '../types';

type LogCallback = (log: VsockLogEntry) => void;
type StatusCallback = (connected: boolean) => void;
type TerminalCallback = (text: string) => void;

/** Manages real-time streaming of vsock packet frames from the backend */
export class VsockLogSocket {
  private socket: WebSocket | null = null;
  private logCallbacks: Set<LogCallback> = new Set();
  private statusCallbacks: Set<StatusCallback> = new Set();
  private reconnectTimer: number | null = null;
  private isExplicitlyClosed = false;

  constructor() {
    this.connect();
  }

  private connect() {
    if (this.isExplicitlyClosed) return;

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    // When proxied via Vite dev server, /api/ws/logs routes to localhost:3001
    const wsUrl = `${protocol}//${window.location.host}/api/ws/logs`;

    try {
      this.socket = new WebSocket(wsUrl);

      this.socket.onopen = () => {
        this.notifyStatus(true);
        if (this.reconnectTimer) {
          clearTimeout(this.reconnectTimer);
          this.reconnectTimer = null;
        }
      };

      this.socket.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data && data.timestamp && data.message) {
            this.notifyLogs(data as VsockLogEntry);
          }
        } catch {
          // Non-JSON or handshake packet
        }
      };

      this.socket.onclose = () => {
        this.notifyStatus(false);
        this.scheduleReconnect();
      };

      this.socket.onerror = () => {
        this.notifyStatus(false);
      };
    } catch {
      this.notifyStatus(false);
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    if (this.isExplicitlyClosed || this.reconnectTimer) return;
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, 3000);
  }

  public subscribe(cb: LogCallback): () => void {
    this.logCallbacks.add(cb);
    return () => this.logCallbacks.delete(cb);
  }

  public onStatusChange(cb: StatusCallback): () => void {
    this.statusCallbacks.add(cb);
    return () => this.statusCallbacks.delete(cb);
  }

  private notifyLogs(log: VsockLogEntry) {
    this.logCallbacks.forEach((cb) => {
      try {
        cb(log);
      } catch (err) {
        console.error('Error in log subscriber callback:', err);
      }
    });
  }

  private notifyStatus(connected: boolean) {
    this.statusCallbacks.forEach((cb) => {
      try {
        cb(connected);
      } catch (err) {
        console.error('Error in status callback:', err);
      }
    });
  }

  public close() {
    this.isExplicitlyClosed = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }
}

/** Manages interactive VM terminal WebSocket session */
export class TerminalSocket {
  private socket: WebSocket | null = null;
  private messageCallbacks: Set<TerminalCallback> = new Set();
  private isOpen = false;
  private vmId: string;

  constructor(vmId: string) {
    this.vmId = vmId;
    this.connect();
  }

  public isConnected(): boolean {
    return this.isOpen;
  }

  private connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/api/ws/terminal/${encodeURIComponent(this.vmId)}`;

    try {
      this.socket = new WebSocket(wsUrl);

      this.socket.onopen = () => {
        this.isOpen = true;
      };

      this.socket.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data && data.text) {
            this.notifyMessage(data.text);
          }
        } catch {
          this.notifyMessage(event.data);
        }
      };

      this.socket.onclose = () => {
        this.isOpen = false;
      };

      this.socket.onerror = () => {
        this.isOpen = false;
      };
    } catch {
      this.isOpen = false;
    }
  }

  public sendCommand(cmd: string) {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(cmd);
    }
  }

  public onMessage(cb: TerminalCallback): () => void {
    this.messageCallbacks.add(cb);
    return () => this.messageCallbacks.delete(cb);
  }

  private notifyMessage(text: string) {
    this.messageCallbacks.forEach((cb) => {
      try {
        cb(text);
      } catch (err) {
        console.error('Error in terminal callback:', err);
      }
    });
  }

  public close() {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
    this.isOpen = false;
  }
}
