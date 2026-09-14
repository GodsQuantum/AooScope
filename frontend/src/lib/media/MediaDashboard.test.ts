import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import MediaDashboard from './MediaDashboard.svelte';

describe('MediaDashboard', () => {
  it.each(['idle', 'offline'] as const)('renders an honest %s state without invented activity', (mode) => {
    render(MediaDashboard, { event: { mode }, onorbit: () => {}, onpreview: () => {} });
    expect(screen.getByText(mode === 'offline' ? 'Sources média hors ligne' : 'Média inactif')).toBeTruthy();
    expect(screen.queryByText(/Sample media|The Expanse|MB\/s/)).toBeNull();
  });

  it('renders only the supplied playing activity', () => {
    render(MediaDashboard, { event: { mode: 'playing', title: 'Real episode', progress_pct: 41, provider_chain: ['Jellyfin'] }, onorbit: () => {}, onpreview: () => {} });
    expect(screen.getByText('Real episode')).toBeTruthy();
    expect(screen.getByText(/Jellyfin/)).toBeTruthy();
    expect(screen.queryByText(/The Expanse|qBittorrent|MB\/s/)).toBeNull();
  });

  it('renders incoming metadata only when present in the event', () => {
    render(MediaDashboard, { event: { mode: 'incoming', title: 'Real download', progress_pct: 82, eta_minutes: 7, speed_bytes_s: 2_400_000, provider_chain: ['Sonarr', 'qBittorrent'] }, onorbit: () => {}, onpreview: () => {} });
    expect(screen.getByText('Real download')).toBeTruthy();
    expect(screen.getByText(/Sonarr \+ qBittorrent · ETA 7 min · 2 MB\/s/)).toBeTruthy();
    expect(screen.queryByText(/Sample media|The Expanse/)).toBeNull();
  });
});
