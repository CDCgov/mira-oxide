# Color Palette

This document defines the official color palette for mira-oxide. **Only these hex values may be used across the project.**

Grouped by palette families (dark → light).

## Blue

| Hex Code | Usage |
|----------|-------|
| `#032659` | Darkest - Primary text, headings |
| `#0057B7` | Dark - Secondary text, links |
| `#3382CF` | Mid - Interactive elements |
| `#5796D9` | Mid-light - Hover states |
| `#87B5E3` | Light - Highlights |
| `#B8D4ED` | Light-mid - Borders |
| `#DBE8F7` | Lighter - Backgrounds |
| `#ECF5FF` | Lightest - Subtle backgrounds |

## Teal

| Hex Code | Usage |
|----------|-------|
| `#125261` | Darkest - Strong accents |
| `#0081A1` | Dark - Success states (dark) |
| `#00B1CE` | Mid - Success states (light) |
| `#7DDEEC` | Mid-light - Info highlights |
| `#AEECF2` | Light - Soft accents |
| `#D5F7F9` | Light-mid - Borders, subtle backgrounds |
| `#EAF8F9` | Lighter - Card backgrounds |
| `#F4FCFC` | Lightest - Page backgrounds |

## Purple

| Hex Code | Usage |
|----------|-------|
| `#47264F` | Darkest - Primary brand, gradients |
| `#722161` | Dark - Secondary brand, gradients |
| `#8F4A8F` | Mid - Staged/pending states |
| `#B278B2` | Mid-light - Staged highlights |
| `#D1ADD4` | Light - Soft accents |
| `#E8D6EB` | Light-mid - Subtle backgrounds |
| `#F5EBF5` | Lighter - Card backgrounds |
| `#FAF7FB` | Lightest - Page backgrounds |

## Orange

| Hex Code | Usage |
|----------|-------|
| `#975722` | Darkest - Warning text |
| `#DE8A05` | Dark - Warning states |
| `#FFB24D` | Mid - Warning highlights |
| `#FABF61` | Mid-light - Warning accents |
| `#FCCF85` | Light - Soft warnings |
| `#FCDBA6` | Light-mid - Warning backgrounds |
| `#FCEBC9` | Lighter - Subtle warnings |
| `#FDF7EB` | Lightest - Warning backgrounds |

## Coral

| Hex Code | Usage |
|----------|-------|
| `#944521` | Darkest - Secondary error text |
| `#FB7E38` | Dark - Alert states |
| `#DB5E2E` | Mid - Alert highlights |
| `#FF9C63` | Mid-light - Alert accents |
| `#FCBF9C` | Light - Soft alerts |
| `#FFD9C4` | Light-mid - Alert backgrounds |
| `#FFEBE0` | Lighter - Subtle alerts |
| `#FEF7F3` | Lightest - Alert backgrounds |

## Red

| Hex Code | Usage |
|----------|-------|
| `#660F14` | Darkest - Error text |
| `#CC1B22` | Dark - Error states, critical alerts |
| `#961C1C` | Mid-dark - Error emphasis |
| `#F0695E` | Mid - Error highlights |
| `#F5968F` | Mid-light - Error accents |
| `#FCBDB5` | Light - Soft errors |
| `#FCDEDB` | Light-mid - Error backgrounds |
| `#FCF2F1` | Lightest - Error backgrounds |

## Usage Guidelines

### Status Colors

- **Success/Completed**: Teal gradient (`#0081A1` → `#00B1CE`)
- **Running/In Progress**: Blue gradient (`#0057B7` → `#3382CF`)
- **Error/Failed**: Red gradient (`#CC1B22` → `#F0695E`)
- **Staged/Pending**: Purple gradient (`#8F4A8F` → `#B278B2`)

### Backgrounds

- **Primary backgrounds**: Lightest shades (e.g., `#F4FCFC`, `#ECF5FF`, `#FAF7FB`)
- **Card backgrounds**: Light-mid shades (e.g., `#EAF8F9`, `#DBE8F7`, `#E8D6EB`)
- **Borders**: Light-mid to light shades (e.g., `#D5F7F9`, `#B8D4ED`)

### Text

- **Primary text**: Darkest shades (e.g., `#032659`, `#125261`)
- **Secondary text**: Dark to mid shades (e.g., `#0057B7`, `#0081A1`)

### Brand Gradients

- **Primary brand gradient**: `#47264F` → `#722161` (Purple dark → mid)
- **Alternate gradient**: `#032659` → `#0057B7` (Blue darkest → mid)

## Implementation

When implementing colors in code:

1. **Use exact hex values** - Do not modify or approximate
2. **Choose semantically appropriate colors** - Match color family to purpose
3. **Maintain consistency** - Similar UI elements should use similar colors
4. **Test accessibility** - Ensure sufficient contrast for text readability
5. **Document usage** - Add comments explaining color choices in complex UI

## Notes

- All color values are case-insensitive for hex codes
- When using transparency, apply rgba() or opacity to these base colors
- For gradients, use adjacent shades from the same family when possible
- This palette is designed to be WCAG AA compliant when used appropriately
