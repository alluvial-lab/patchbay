export function requireSafeGatewayBaseUrl(url: URL, name: string): void {
  const validShape = ["http:", "https:"].includes(url.protocol)
    && !url.username
    && !url.password
    && !url.search
    && !url.hash;
  if (!validShape || (url.protocol === "http:" && !isLoopbackHostname(url.hostname))) {
    throw new Error(
      `${name} must be a credential-free http(s) URL; HTTPS is required except for loopback-only local development`,
    );
  }
}

function isLoopbackHostname(hostname: string): boolean {
  return hostname === "localhost"
    || hostname === "[::1]"
    || /^127(?:\.[0-9]{1,3}){3}$/.test(hostname);
}
