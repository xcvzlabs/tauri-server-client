import * as v from 'valibot';

const serverUrlInputSchema = v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter a server address'));

export type NormalizedServerUrl = {
  httpBase: string;
  wsBase: string;
};

export function normalizeServerUrl(input: string): NormalizedServerUrl {
  const trimmed = v.parse(serverUrlInputSchema, input);
  const withScheme = /^https?:\/\//i.test(trimmed) ? trimmed : `http://${trimmed}`;

  let url: URL;
  try {
    url = new URL(withScheme);
  } catch {
    throw new Error('Enter a valid server address, e.g. 192.168.1.10:8080');
  }

  const wsProtocol = url.protocol === 'https:' ? 'wss:' : 'ws:';

  return {
    httpBase: url.origin,
    wsBase: `${wsProtocol}//${url.host}`,
  };
}
