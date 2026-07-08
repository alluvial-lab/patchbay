---
source_handle: tanstack-query-docs
fetched: 2026-07-07
source_url: https://tanstack.com/query/latest/docs/framework/react/guides/important-defaults
additional_source_urls:
  - https://tanstack.com/query/latest/docs/framework/react/guides/network-mode
  - https://tanstack.com/query/latest/docs/framework/react/guides/window-focus-refetching
provenance: source-direct
---

# TanStack Query documentation

## Structural metadata

- Source type: official TanStack Query React documentation pages.
- Fetched representation: HTML rendered to text with `lynx`.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/tanstack-*.txt`.

## Paraphrased source summary

TanStack Query documents cache, staleness, refetch, focus, reconnect, and network-mode behavior for server-state queries and mutations. It treats cached data as stale by default, refetches stale queries on mount/window focus/network reconnect unless configured otherwise, and exposes network pause/fetch status through network modes.

## Key passages

1. The Important Defaults guide says TanStack Query uses "aggressive but sane defaults" and by default `useQuery`/`useInfiniteQuery` query instances consider cached data stale.

2. The docs say `staleTime` can be configured globally or per query; longer `staleTime` reduces refetching, `Infinity` prevents refetch until manual invalidation, and `'static'` prevents refetch even after manual invalidation.

3. The docs say stale queries are refetched automatically in the background when new query instances mount, the window is refocused, or the network is reconnected.

4. The docs say refetch points can be customized with `refetchOnMount`, `refetchOnWindowFocus`, and `refetchOnReconnect`.

5. The docs say inactive query results remain cached by default and are garbage-collected after five minutes.

6. The Network Mode guide says TanStack Query provides three network modes for how queries and mutations behave without network connection, configurable per query/mutation or globally.

7. In default `online` mode, queries and mutations do not fire without network connection; a query may stay in its state while `fetchStatus` indicates `fetching`, `paused`, or `idle`.

8. The Network Mode guide says a first-time query can be `state: 'pending'` with `fetchStatus: 'paused'` when there is no network connection, so pending alone is not enough for a loading spinner.

9. If a query starts online and goes offline while fetching, TanStack Query pauses retry and continues once network returns; this is distinct from `refetchOnReconnect`.

10. In `offlineFirst`, TanStack Query runs the query function once, then pauses retries; this fits cases where the first fetch might succeed from offline storage or HTTP cache but a cache miss fails.

11. The Window Focus guide says if query data is stale and a user leaves and returns, TanStack Query automatically requests fresh data in the background; this can be disabled globally or per query.

12. The Window Focus guide says `focusManager.setEventListener` can replace default focus events and supplies a callback to fire when the window is focused.
