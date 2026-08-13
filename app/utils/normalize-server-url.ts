import * as v from 'valibot';

const serverURLInputSchema = v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter a server address'));

export function normalizeServerURL(input: string): NormalizedServerURL {
  const trimmed = v.parse(serverURLInputSchema, input);
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
