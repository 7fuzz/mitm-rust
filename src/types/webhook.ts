export interface WebhookEndpoint {
  id: string;
  name: string;
  pathSlug: string;
  mockStatus: number;
  mockHeaders: string;
  mockBody: string;
  autoForwardUrl?: string | null;
  isActive: boolean;
  createdAt?: number;
}

export interface WebhookDelivery {
  id: string;
  endpointId?: string | null;
  method: string;
  path: string;
  headers: string; // JSON string of headers map
  queryParams: string;
  body: string;
  clientIp?: string | null;
  forwarded: boolean;
  forwardStatus?: number | null;
  forwardResponseBody?: string | null;
  timestamp: number;
}

export interface WebhookTriggerRequest {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: string;
  signatureSecret?: string;
  providerPreset?: 'stripe' | 'github' | 'shopify' | 'slack' | 'custom';
}

export interface WebhookForwardResult {
  success: boolean;
  statusCode?: number | null;
  responseBody?: string | null;
  error?: string | null;
}

export interface WebhookListenerConfig {
  port: number;
  isRunning: boolean;
}

export interface WebhookTemplate {
  id: string;
  name: string;
  provider: 'stripe' | 'github' | 'shopify' | 'slack' | 'custom';
  defaultHeader: string;
  sampleBody: string;
}

export const WEBHOOK_TEMPLATES: WebhookTemplate[] = [
  {
    id: 'stripe-payment-success',
    name: 'Stripe: payment_intent.succeeded',
    provider: 'stripe',
    defaultHeader: 'Stripe-Signature',
    sampleBody: JSON.stringify({
      id: 'evt_1M00002eZvKYlo2C00000000',
      object: 'event',
      api_version: '2022-11-15',
      created: Math.floor(Date.now() / 1000),
      type: 'payment_intent.succeeded',
      data: {
        object: {
          id: 'pi_3M00002eZvKYlo2C00000000',
          object: 'payment_intent',
          amount: 2000,
          currency: 'usd',
          status: 'succeeded'
        }
      }
    }, null, 2)
  },
  {
    id: 'github-push',
    name: 'GitHub: push event',
    provider: 'github',
    defaultHeader: 'X-Hub-Signature-256',
    sampleBody: JSON.stringify({
      ref: 'refs/heads/main',
      before: '0000000000000000000000000000000000000000',
      after: '611f71b87a87d096181e1882b4a169b18ed39fa6',
      repository: {
        name: 'mitm-rust',
        full_name: 'user/mitm-rust',
        private: false
      },
      pusher: {
        name: 'developer',
        email: 'developer@example.com'
      }
    }, null, 2)
  },
  {
    id: 'shopify-order-created',
    name: 'Shopify: orders/create',
    provider: 'shopify',
    defaultHeader: 'X-Shopify-Hmac-Sha256',
    sampleBody: JSON.stringify({
      id: 82098291194,
      email: 'customer@example.com',
      total_price: '199.99',
      subtotal_price: '180.00',
      currency: 'USD',
      financial_status: 'paid',
      line_items: [
        {
          id: 1,
          title: 'Premium Developer License',
          price: '199.99',
          quantity: 1
        }
      ]
    }, null, 2)
  },
  {
    id: 'slack-message',
    name: 'Slack: message event',
    provider: 'slack',
    defaultHeader: 'X-Slack-Signature',
    sampleBody: JSON.stringify({
      token: 'XXYYZZ',
      team_id: 'T0001',
      api_app_id: 'A0001',
      event: {
        type: 'message',
        channel: 'C2147483705',
        user: 'U2147483697',
        text: 'Hello from Webhook tester!'
      }
    }, null, 2)
  },
  {
    id: 'custom-json',
    name: 'Custom: Event Payload',
    provider: 'custom',
    defaultHeader: 'X-Signature-256',
    sampleBody: JSON.stringify({
      event: 'user.created',
      timestamp: Math.floor(Date.now() / 1000),
      data: {
        userId: 'usr_12345',
        role: 'admin'
      }
    }, null, 2)
  }
];
