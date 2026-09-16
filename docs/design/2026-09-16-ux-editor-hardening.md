# AooScope Visual Studio UX hardening

## Approved scope

- Make the editor canvas visually faithful to the rendered LCD page: decorative gauges/bars do not inject metric labels or values; text/value layers own their text.
- Make enabled/disabled state immediately visible in the page sequence, distinct from current selection, with direct toggle access and no ambiguous wrapping.
- Make drag/resize geometry integral at every viewport width so Preview never fails because scaled pointer math emits floats.
- Surface backend error messages instead of generic HTTP status only.
- Normalize disk SMART temperatures from explicit fields, ATA attributes 194/190, and NVMe SMART text when Proxmox omits a top-level temperature.
- Keep the existing dark/cyan visual identity, but reduce density and use progressive disclosure for secondary controls.
- Validate 1600/1024/800 widths, all main tabs, Home/Compute/Storage previews, shadow production container, CI/GHCR, and production before completion.
