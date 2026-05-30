export interface Traffic {
  phase: string;
  id: string;
  method: string;
  url: string;
  host: string;
  status_code: number;
  request_headers: Record<string, string>;
  response_headers: Record<string, string>;
  request_body: string;
  response_body: string;

  // Add this line (the '?' makes it optional for backwards compatibility)
  is_intercepted?: boolean;
  intercepted_at?: number;
  group?: string;
  hit_count?: number;
}
