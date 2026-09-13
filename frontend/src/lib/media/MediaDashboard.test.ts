import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import MediaDashboard from './MediaDashboard.svelte';

describe('MediaDashboard', () => {
  it('keeps poster-first playing and provider-rich incoming cards offline', () => {
    render(MediaDashboard, { onorbit: () => {}, onpreview: () => {} });
    expect(screen.getByText('Lecture en cours')).toBeTruthy();
    expect(screen.getByText(/Jellyfin · demo/)).toBeTruthy();
    expect(screen.getByText(/Sonarr \+ qBittorrent/)).toBeTruthy();
    expect(screen.getByText(/ETA 12 min/)).toBeTruthy();
  });
});
