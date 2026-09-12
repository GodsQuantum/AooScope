import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import DisplayStatus from './DisplayStatus.svelte';
import type { components } from '../api/schema';

type StatusDto = components['schemas']['StatusDto'];

describe('DisplayStatus', () => {
  it('renders brightness and connected state', () => {
    const status: StatusDto = {
      version: '0.3.0-dev', brightness: 73, native_brightness: false,
      device_present: true, updated_unix: 1789200000
    };
    render(DisplayStatus, { props: { status } });
    expect(screen.getByText('73%')).toBeTruthy();
    expect(screen.getByText('Connected')).toBeTruthy();
  });
});
