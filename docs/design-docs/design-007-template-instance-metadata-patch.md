# Template Instance Metadata Patch

**Status:** Active Patch  
**Target:** markbase CLI  
**Related docs:** `docs/design-docs/design-006-template-system.md`, `docs/design-docs/design-004-note-verify.md`, `ARCHITECTURE.md`, `README.md`

## Scope

This document is a patch design for the active template subsystem.

It exists to define a targeted behavior change before that change is merged into `docs/design-docs/design-006-template-system.md`.

It covers:

- how template-authored instance defaults are represented
- why template files must stop reusing instance frontmatter as template metadata
- how `note new --template` materializes instance frontmatter
- how `templates` is derived
- how `note verify` changes when instance defaults move under `_schema.instance`
- migration and compatibility rules during the transition

It does not replace the full template-system contract. All `design-006` behavior remains active except where this patch explicitly overrides it.

## Patch Intent

Current template behavior uses outer frontmatter for two incompatible roles:

- metadata stored on the template file itself
- instance-facing fields copied into created notes

That coupling creates a semantic conflict. A template file such as `templates/company_customer.md` is not itself a `company` entity and has not itself been created from the `company_customer` template, yet the current model encourages authors to write fields like:

```yaml
type: company
templates:
  - "[[company_customer]]"
```

This patch removes that conflict by moving instance-default metadata under `_schema.instance`.

## Authority

Until this patch is implemented and merged into `design-006`, use this document as the source of truth for the specific topics below:

- representation of template-authored instance defaults
- whether `templates` is template-authored or system-derived
- which template fields are copied into created instances
- what role `_schema.instance` has in `note verify`

If this patch conflicts with `design-006` on those topics, this patch wins.

## New Template Model

Templates still have two top-level authoring surfaces:

1. outer frontmatter
2. Markdown body

Within outer frontmatter, `_schema` remains the template-owned metadata object.

This patch adds a new `_schema.instance` object:

- `_schema.instance` contains instance-facing frontmatter defaults to materialize during `note new --template`
- `_schema.properties` continues to define validation and field-shape constraints
- outer frontmatter no longer serves as the instance skeleton for note creation

### `_schema.instance`

`_schema.instance` is an object whose keys are note frontmatter fields and whose values are the literal defaults to write into created instances.

Example:

```yaml
_schema:
  location: entities/company/
  properties:
    description:
      type: text
      description: 一句话摘要
    status:
      type: text
      enum: ["Lead", "Active", "Paused", "Closed Won", "Closed Lost", "Other"]
      default: "Lead"
  instance:
    type: company
    tags: []
    status: Lead
```

`_schema.instance` is a creation-time materialization surface, not a general exact-match verification surface.

That distinction is intentional:

- fields in `_schema.instance` may be stable identity fields such as `type`
- fields in `_schema.instance` may also be mutable starting values such as `status: Lead`

`note verify` therefore must not treat `_schema.instance` as "all created notes must always equal these literals".

## `templates` Is System-Derived

`templates` is no longer a template-authored instance field.

Instead, `note new --template <name>` must always write:

```yaml
templates:
  - "[[<name>]]"
```

Rules:

- authors must not declare `templates` inside `_schema.instance`
- outer frontmatter `templates` on the template file has no instance-creation meaning
- the created note always receives the template link for the selected template name

This makes `templates` a system-derived binding field instead of duplicated author input.

## Overrides To `design-006`

This patch replaces the following `design-006` assumptions:

- outer frontmatter is no longer the primary instance skeleton copied into created notes
- only `_schema` is no longer sufficient to describe template-only metadata; `_schema.instance` is now part of the creation contract
- `note new --template` no longer copies arbitrary non-`_schema` outer frontmatter fields into instances

The new split is:

- outer frontmatter: metadata about the template file itself, if any
- `_schema`: template-owned metadata
- `_schema.instance`: instance defaults for creation
- Markdown body: instance body content copied during creation

This patch also replaces one verification-side assumption:

- template outer frontmatter is no longer the source of instance literal-match checks

## Creation Contract

After this patch, `note new --template <name>` must:

1. Load and normalize the template.
2. Read `_schema.location` for output directory selection.
3. Build instance frontmatter from `_schema.instance`.
4. Auto-inject `templates: ["[[<name>]]"]`.
5. Ensure instance `description` exists and is a string, preserving the current global description contract.
6. Copy the Markdown body into the instance.
7. Replace supported variables such as `{{name}}`, `{{date}}`, `{{time}}`, and `{{datetime}}`.

`_schema` itself is never written into the instance.

## Normalization Rules

The normalized template read model must gain these behaviors:

1. Ensure `_schema.instance` exists and is an object. If absent or non-object, normalize it to an empty object.
2. Continue normalizing `_schema.required` and `_schema.properties.description`.
3. Ensure the instance materialization path produces a string `description` field, even if `_schema.instance.description` is absent or non-string.
4. Do not treat arbitrary outer frontmatter fields as instance defaults.

This keeps the existing description invariant without preserving the old outer-frontmatter coupling.

## `default` Versus `instance`

This patch keeps `_schema.properties.<field>.default` and `_schema.instance.<field>` distinct.

- `_schema.properties.<field>.default` is schema-level fallback metadata
- `_schema.instance.<field>` is the concrete value materialized during note creation

`note new --template` does not automatically convert schema `default` into `_schema.instance` values.

The intended authoring model is:

- use `_schema.instance` when the template should materialize a starting value into a newly created note
- use `_schema.properties` and `_schema.required` when the field should be validated later
- use both when a field needs an initial value and a continuing validation rule

### Stable Identity Rule

Some instance fields are not merely starting values. They define the continuing identity of the note class created by the template.

Examples include:

- `type`
- any future field whose meaning is "this note is a member of category X" rather than "this note starts in state X"

For such stable identity fields, the template contract must be expressed in both places:

- `_schema.instance.<field>` defines the value written during note creation
- `_schema.required` and `_schema.properties.<field>` define the continuing verification rule

This is a mandatory rule, not a style preference.

In particular:

- if a template intends all created notes to remain `type: company`, it must set `_schema.instance.type: company`
- and it must also require `type` and constrain it through `_schema.properties.type`
- a seed-only `type` value in `_schema.instance` without matching verification constraints is invalid template modeling

Examples:

- `type: company`
  Put `type: company` in `_schema.instance` so new notes start with the right type.
  Also declare `type` in `_schema.required` and `_schema.properties.type.enum: [company]` if verification should enforce that the note remains a company note.
- `status: Lead`
  Put `status: Lead` in `_schema.instance` if that is the desired initial state.
  Keep allowed lifecycle values in `_schema.properties.status.enum`.
  `note verify` should allow later values such as `Active` or `Closed Won`.

## `note verify` Contract Changes

This patch changes how verification interprets template files.

### What Stays The Same

`note verify` still:

- starts from the instance note's `templates` field
- resolves template files from `templates/<name>.md`
- validates `_schema.location`
- validates `_schema.required`
- validates `_schema.properties`
- validates template body `.base` embed requirements

### What Stops Happening

`note verify` must stop treating template outer frontmatter as instance literal constraints.

The old behavior being replaced is:

- missing non-`_schema` template field in note frontmatter is an error
- scalar outer-frontmatter mismatch is an error
- list outer-frontmatter containment mismatch is an error

Those checks were only coherent when outer frontmatter doubled as the instance skeleton. After this patch, they are no longer valid.

### What `_schema.instance` Means For Verification

`_schema.instance` affects `note verify` indirectly, not as an exact-equality rule set.

The verifier contract should be:

- `_schema.instance` is the creation blueprint for new notes
- `_schema.instance` is not itself a list of values the instance must always match exactly
- continuing invariants must be expressed through `_schema.required` and `_schema.properties`

In practice:

- if a field appears only in `_schema.instance`, verifier does not require the field to remain equal to that initial value
- if a field appears in `_schema.instance` and is also constrained in `_schema.properties`, verifier enforces the schema constraint rather than the original seed literal
- if a field must be present, `_schema.required` is the source of truth for presence requirements

This separation lets templates seed mutable fields without freezing them forever.

The stable-identity exception is intentional and explicit:

- stable identity fields such as `type` must be modeled as both seed values and continuing constraints
- mutable workflow fields such as `status` may be modeled as seed-only values

### Modeling Guidance For Template Authors

When a field is:

- a creation-only seed value
  Put it in `_schema.instance`.
- a required field with type or enum validation
  Put it in `_schema.required` and `_schema.properties`.
- both a seed value and a continuing constraint
  Put it in both places.

### Example Translation From Legacy Behavior

Legacy template authoring:

```yaml
type: company
status: Lead
tags: []
```

Patched authoring:

```yaml
_schema:
  required:
    - description
    - type
  properties:
    description:
      type: text
    type:
      type: text
      enum: [company]
    status:
      type: text
      enum: ["Lead", "Active", "Paused", "Closed Won", "Closed Lost", "Other"]
  instance:
    type: company
    status: Lead
    tags: []
```

Result:

- new notes start with `type: company` and `status: Lead`
- `note verify` requires `type` to remain `company`
- `note verify` allows `status` to evolve as long as it stays within the declared enum
- `tags: []` is only a creation seed unless separately modeled as a constraint

## Compatibility And Migration

This patch intentionally supports a staged migration.

### Authoring Guidance

New templates should:

- put validation and routing data in `_schema`
- put instance defaults in `_schema.instance`
- omit outer frontmatter instance fields such as `type`, `tags`, and `templates`

### Transitional Read Behavior

During implementation rollout, markbase may support compatibility fallback from legacy outer frontmatter instance fields to `_schema.instance`.

If compatibility fallback is retained temporarily, it must follow these rules:

- `_schema.instance` wins when both old and new forms are present
- legacy outer frontmatter `templates` must not be copied into created instances
- compatibility behavior is transitional and should be removed after vault migration

### Template Verification Boundary

This patch changes the verification design inputs, even if the `note verify` CLI shape stays the same.

Implementation following this patch should ensure template files are not treated as ordinary note instances for self-verification scenarios merely because they live in the index and carry frontmatter.

If command-visible `note verify <template-name>` behavior changes, merge that final command contract into `design-004` together with the implementation.

## Example

The earlier company template example should move from:

```yaml
type: company
templates:
  - "[[company_customer]]"
tags: []
```

to:

```yaml
_schema:
  location: company/
  required:
    - description
    - type
  properties:
    description:
      type: text
      description: 一句话说明这个 note 是什么
    type:
      type: text
      enum: [company]
  instance:
    type: company
    tags: []
```

When creating `markbase note new acme --template company_customer`, the resulting instance frontmatter should include:

```yaml
type: company
templates:
  - "[[company_customer]]"
tags: []
description: ""
```

## Non-Goals

This patch does not define:

- multi-template creation semantics
- schema-default materialization beyond the explicit `_schema.instance` surface
- changes to template body directive execution
- a full `note verify` redesign beyond the `_schema.instance` transition

## Merge Plan

After implementation lands and compatibility behavior is settled:

1. merge this patch into `docs/design-docs/design-006-template-system.md`
2. merge the verification-side semantics from this patch into `docs/design-docs/design-004-note-verify.md`
3. update `README.md` if author-facing template examples or creation semantics changed
4. remove or archive this patch document once its content is absorbed
