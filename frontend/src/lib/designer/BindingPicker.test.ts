import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import BindingPicker from './BindingPicker.svelte';

describe('BindingPicker', () => {
  it('groups human metrics by provider and category without a free-text id field', () => {
    render(BindingPicker, { props: {
      widgetType: 'gauge', value: 'aooscope_pve_cpu_pct', metrics: [{
        id: 'aooscope_pve_cpu_pct', label: 'Utilisation CPU', provider_name: 'Proxmox',
        category: 'CPU', value: 42, demo_value: 12, unit: '%', recommended_widgets: ['gauge'], online: true
      }]
    }});
    expect(screen.getByText('Proxmox / CPU')).toBeTruthy();
    expect(screen.getByText('Utilisation CPU')).toBeTruthy();
    expect(screen.getByText('42 %')).toBeTruthy();
    expect(screen.queryByRole('textbox', { name: /binding id/i })).toBeNull();
  });
});
