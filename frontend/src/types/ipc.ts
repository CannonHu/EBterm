export type DataBits = 'seven' | 'eight';

export type Parity = 'none' | 'odd' | 'even';

export type StopBits = 'one' | 'two';

export type FlowControl = 'none' | 'software' | 'hardware';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

export interface IpcError {
  code: string;
  message: string;
  details?: string;
}

export type WriteError = 
  | { type: 'disconnected' }
  | { type: 'timeout' }
  | { type: 'io_error'; message: string };

export interface SerialParams {
  port: string;
  baud_rate: number;
  data_bits: DataBits;
  parity: Parity;
  stop_bits: StopBits;
  flow_control: FlowControl;
}

export interface TelnetParams {
  host: string;
  port: number;
  connect_timeout_secs: number;
}

export type ConnectionParams =
  | { type: 'serial' } & SerialParams
  | { type: 'telnet' } & TelnetParams;

export interface ConnectionStats {
  bytes_sent: number;
  bytes_received: number;
  packets_sent: number;
  packets_received: number;
}

export interface SessionInfo {
  id: string;
  name: string;
  connection_type: string;
  status: ConnectionStatus;
  created_at: string;
  last_activity?: string;
  stats: ConnectionStats;
  logging_enabled: boolean;
  log_file_path?: string;
}

export type LogDirection = 'input' | 'output';

export interface LoggingStatus {
  enabled: boolean;
  file_path?: string;
  bytes_logged_input: number;
  bytes_logged_output: number;
  started_at?: string;
}

export interface CommandInfo {
  index: number;
  name: string;
  description?: string;
  content_preview: string;
  line_number: number;
}

export interface SerialPortInfo {
  port_name: string;
  port_type: string;
  vendor_id?: number;
  product_id?: number;
  serial_number?: string;
  manufacturer?: string;
  product?: string;
}

export interface LogEntry {
  timestamp: number;
  session_id: string;
  direction: LogDirection;
  data: number[];
}

export interface CommandResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface TabState {
  id: string;
  sessionId: string | null;
  title: string;
  isActive: boolean;
  isConnecting: boolean;
}

export interface TerminalUIState {
  showTimestamps: boolean;
  isSearchOpen: boolean;
  isConfigPanelOpen: boolean;
}
