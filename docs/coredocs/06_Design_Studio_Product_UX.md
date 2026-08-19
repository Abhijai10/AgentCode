# AgentCode  
# 06 — Design Studio & Product UX Architecture

**Document Status:** V1 — Architecture Locked for Initial Implementation + Hardening Revision 2  
**Date:** 19 August 2026  
**Revision:** Hardening Revision 2 — Implementation-Grade Product UX & Design Studio Contracts  
**Project:** AgentCode  
**Document Type:** Core Architecture Specification  

**Depends On:**
- `01 — Model, Provider, Routing & Reliability Architecture`
- `02 — Code Intelligence, Context & Persistent Memory Architecture`
- `03 — Autonomy Kernel & Agent Runtime Architecture`
- `04 — Tool, Edit, Git, Sandbox, Skills & Hooks Architecture`
- `05 — Verification, Security & Red-Team Architecture`

**Primary Reference Repository Root:**  
`/Volumes/T7 Shield/GitHub-Repos-dependency`

**Scope:** AgentCode desktop UX, minimal Codex-like mission interface, Goal Mode, Discuss Mode, Design Studio, Security Mode entry points, background execution, progressive disclosure, project management, mission status, activity inspection, diffs, terminals, notifications, completion sounds, design generation, Lovable/Emergent-style prompt-to-interface workflows, anti-generic-AI design safeguards, design-system generation, reference-image analysis, browser preview, iterative screenshot feedback, visual QA, accessibility, responsive validation, product-specific design memory, design acceptance gates, user interruption policies and overall interaction principles.

---

# 1. Purpose

AgentCode is intended to become an extremely capable software-engineering system.

The interface should not expose that complexity unnecessarily.

The product experience should feel:

```text
simple
calm
fast
predictable
professional
```

while internally coordinating:

```text
multiple models
providers
tasks
worktrees
tools
security scans
tests
browser sessions
verification
recovery
context retrieval
```

The primary experience should remain:

```text
OPEN PROJECT
     ↓
DESCRIBE GOAL
     ↓
START
     ↓
LEAVE
     ↓
AGENTCODE WORKS
     ↓
NOTIFICATION
     ↓
REVIEW RESULT
```

The UI must not force the user to supervise the autonomous runtime.

---

# 2. Core UX Principle

AgentCode should follow:

> **Minimal surface, maximum capability.**

The application should not resemble:

```text
virtual office

agent avatar simulator

AI employee dashboard

animated worker room

multi-agent office game
```

AgentCode may internally run many workers.

The user should see only the information necessary to understand:

```text
What is AgentCode doing?

How much is complete?

Is anything blocked?

Does AgentCode need me?

What changed?

Did verification pass?
```

---

# 3. No Office Metaphor

V1 explicitly rejects UI patterns such as:

```text
agent avatars

office rooms

desks

characters walking around

fake employee personas

animated collaboration scenes
```

These may look interesting but provide little engineering value.

AgentCode is:

```text
engineering software
```

not:

```text
a simulation of an engineering company.
```

---

# 4. Product Personality

AgentCode should feel:

```text
precise

quiet

technical

premium

confident

minimal

transparent when inspected

non-distracting
```

It should not feel:

```text
toy-like

overly colorful

gamified

corporate-dashboard heavy

chatbot-only

AI-template generated
```

---

# 5. Primary Interaction Modes

AgentCode V1 should expose four main modes:

```text
GOAL

DISCUSS

DESIGN

SECURITY
```

These are user-facing entry modes.

Internally they all use the same:

```text
Kernel

Model Broker

Code Intelligence

Tools

Skills

Verification
```

They do not create four separate products.

---

# 6. GOAL Mode

Goal Mode is the flagship AgentCode experience.

Example:

```text
┌─────────────────────────────────────────────┐
│ AgentCode                                   │
│                                             │
│ Project: AstraFlow                         │
│                                             │
│ What should AgentCode accomplish?          │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ Make the authentication system          │ │
│ │ production-ready and fix all issues.   │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│                        [ Start Mission ]    │
└─────────────────────────────────────────────┘
```

Goal Mode should accept:

```text
short goals

large goals

bug descriptions

audit requests

feature requests

refactors

production-readiness goals
```

The user should not need to manually decompose the task.

---

# 7. Goal Mode Philosophy

After mission start:

```text
AgentCode owns execution.
```

The user should not repeatedly need to type:

```text
continue

yes

go ahead

run tests

fix remaining issues

please finish everything
```

The Kernel handles continuity.

---

# 8. Default Mission Screen

After starting:

```text
┌──────────────────────────────────────────────────────┐
│ AgentCode        AstraFlow            ● Working     │
├──────────────────────────────────────────────────────┤
│                                                      │
│ Production-ready authentication                     │
│                                                      │
│ ███████████████░░░░░░░░░░░  58%                    │
│                                                      │
│ Current                                              │
│ Implementing session invalidation                    │
│                                                      │
│ ✓ Repository analysed                               │
│ ✓ Authentication flows mapped                       │
│ ✓ Token refresh repaired                            │
│ → Session invalidation                              │
│ ○ Integration verification                          │
│ ○ Final audit                                       │
│                                                      │
│ No action required                                  │
│                                                      │
│ [Changes] [Activity] [Details]                      │
└──────────────────────────────────────────────────────┘
```

Only high-value status appears by default.

---

# 9. Progressive Disclosure

The UI should expose deeper information in layers.

## Level 1 — Default

```text
mission

progress

current task

blocker

completion status
```

## Level 2 — Activity

```text
tasks

recent actions

tests

workers

verification
```

## Level 3 — Engineering Detail

```text
raw terminal

tool calls

model/provider

context pack

worktrees

routing decisions

logs
```

The user chooses when to descend into complexity.

---

# 10. Do Not Fake Progress

Mission percentages must not be arbitrary animation.

Progress should derive from Kernel state such as:

```text
requirements

task weights

verified milestones

critical path

integration status
```

If accurate percentage cannot be calculated:

```text
Working
```

is preferable to fake:

```text
73%.
```

---

# 11. Progress Representation

Possible inputs:

```text
required tasks completed

requirement coverage

verification completion

critical-path progression

task weights

blocked work
```

Progress should distinguish:

```text
implemented
```

from:

```text
verified.
```

---

# 12. Mission Status States

Possible visible states:

```text
Preparing

Analysing

Planning

Working

Testing

Verifying

Security Audit

Final Audit

Needs You

Paused

Blocked

Complete
```

The UI should not expose every internal Kernel state by default.

---

# 13. "Needs You"

This state is important.

AgentCode should interrupt the user only for meaningful decisions such as:

```text
production deployment

missing login

paid-budget approval

destructive migration

irreversible external operation

uncertain product decision
```

Not:

```text
read file?

run grep?

run npm test?

edit src/auth.ts?
```

---

# 14. Mission Detail Screen

An expanded mission can expose:

```text
Goal

Requirements

Plan

Tasks

Changes

Verification

Security

Models

Context

Timeline
```

These should appear as compact tabs or expandable surfaces.

---

# 15. Activity View

Example:

```text
01:14  ✓ Analysed authentication architecture

01:16  ✓ Found 3 session-management defects

01:21  ✓ Fixed token refresh race

01:23  → Running affected tests

01:25  ⚠ NVIDIA unavailable

01:25  ✓ Continued on ModelScope

01:29  → Implementing logout invalidation
```

Provider recovery should be visible but not treated as a crisis if AgentCode recovered automatically.

---

# 16. Changes View

AgentCode needs a high-quality diff viewer.

Display:

```text
changed files

added files

deleted files

renamed files

diff

task responsible

verification state
```

Example:

```text
src/auth/session.ts       +48 -16    ✓ verified

src/auth/token.ts         +27 -8     ✓ verified

tests/session.test.ts     +82 -0     ✓ verified
```

---

# 17. Change Grouping

Changes should be grouped logically by:

```text
task

feature

change set

commit
```

rather than showing one giant mission diff by default.

---

# 18. Terminal View

Terminal access is necessary but should remain secondary.

User may open:

```text
Terminal
```

to see:

```text
tests

builds

server output

commands
```

V1 may embed an xterm-compatible terminal.

Raw logs are not the default mission screen.

---

# 19. Model Transparency

The normal UI should not make provider selection the center of the experience.

Default:

```text
Working
```

Advanced detail may show:

```text
Worker

Qwen3-Coder
via ModelScope
```

or:

```text
Verifier

GPT-OSS
via Cerebras
```

AgentCode should feel model-agnostic.

---

# 20. DISCUSS Mode

Discuss Mode provides a ChatGPT-like engineering conversation connected directly to the repository.

Capabilities:

```text
conversation

repository reading

deep search

architecture reasoning

code explanation

web research

decision support
```

Default Discuss Mode should not mutate code.

---

# 21. Discuss Mode Example

User:

```text
Would Redis actually improve the current session architecture?
```

AgentCode may:

```text
inspect session implementation

inspect database architecture

inspect deployment

inspect requirements

compare approaches
```

then discuss the trade-off.

This is not generic chat.

It is:

```text
repository-aware engineering discussion.
```

---

# 22. Discuss Mode Model Selection

Discuss Mode uses Doc 01 Model Broker.

Routing preference:

```text
strong free cloud reasoning
        ↓
alternate free provider
        ↓
paid route if user policy allows
        ↓
local fallback
```

Local Qwen3 4B can handle lightweight discussion.

It should not be marketed as equivalent to frontier discussion models.

---

# 23. Discuss → Plan

A discussion may become actionable.

UI:

```text
[ Turn Into Plan ]
```

AgentCode extracts:

```text
accepted design

constraints

rejected approaches

requirements

open questions
```

into structured project state.

---

# 24. Plan → Mission

After review:

```text
[ Execute Plan ]
```

creates a Kernel mission.

The user should not need to copy/paste the entire discussion.

---

# 25. Discussion Persistence

Important accepted decisions should move into:

```text
DECISIONS.md

CONTEXT.md

structured Kernel state
```

Normal conversational chatter remains compactable.

---

# 26. DESIGN Mode

Design Mode should offer a Lovable / Emergent / v0-style product-building workflow.

However AgentCode must solve a major weakness of many AI app builders:

> **Generic AI-generated design.**

Design Mode is therefore not simply:

```text
prompt
→ generate Tailwind page
```

It is a complete:

```text
PRODUCT UNDERSTANDING
        ↓
DESIGN DIRECTION
        ↓
DESIGN SYSTEM
        ↓
IMPLEMENTATION
        ↓
BROWSER PREVIEW
        ↓
VISUAL CRITIQUE
        ↓
ITERATION
        ↓
FUNCTIONAL VERIFICATION
```

---

# 27. Design Studio Goal

The user should be able to say:

```text
Redesign this dashboard.

Make it feel like a premium professional VPN product.
Avoid generic AI-generated SaaS design.
Keep the movie-style dark visual identity.
```

AgentCode should transform this into:

```text
design brief

product visual identity

component language

responsive system

working implementation
```

---

# 28. Design Studio Architecture

```text
USER DESIGN GOAL
        │
        ▼
PRODUCT CONTEXT
        │
        ▼
DESIGN DIRECTOR
        │
        ▼
DESIGN BRIEF
        │
        ▼
VISUAL LANGUAGE
        │
        ▼
DESIGN SYSTEM
        │
        ▼
UI IMPLEMENTER
        │
        ▼
BROWSER PREVIEW
        │
        ▼
VISUAL CRITIC
        │
        ▼
ANTI-SLOP REVIEW
        │
        ▼
ACCESSIBILITY / RESPONSIVE QA
        │
        ▼
FUNCTIONAL VERIFIER
        │
       PASS?
      /     \
    NO       YES
    │         │
 ITERATE    COMPLETE
```

---

# 29. Design Director

Design Director is a logical role:

```text
Worker/Planner
+
Design Director Skill
```

not a permanent autonomous persona.

Responsibilities:

```text
understand product

understand audience

derive visual personality

identify information hierarchy

select interaction patterns

define visual language

avoid generic design defaults
```

---

# 30. Product Understanding Before Styling

Before redesigning, AgentCode should determine:

```text
What is the product?

Who uses it?

What actions matter most?

What emotional impression should it create?

How dense should information be?

What existing brand conventions exist?

What UI must remain familiar?
```

Design should follow product intent.

Not vice versa.

---

# 31. Design Brief

Every substantial design task should produce a compact brief.

Example:

```text
Product:
Developer-focused autonomous coding environment

Personality:
Technical
Calm
Premium
Precise

Primary user:
Software developer

Primary action:
Start and monitor autonomous coding mission

Density:
Medium-high

Visual principle:
Complexity hidden behind progressive disclosure

Avoid:
Generic SaaS cards
Decorative gradients
AI chatbot aesthetic
```

---

# 32. Product-Specific Design Grammar

AgentCode should generate or maintain a design grammar containing:

```text
typography

spacing

radius scale

surface hierarchy

elevation

navigation pattern

layout density

color roles

motion principles

iconography

data display

form behavior

interaction feedback
```

This grammar creates coherence across generated screens.

---

# 33. Design Grammar vs Template

The system should not repeatedly start from:

```text
generic dashboard template.
```

Instead:

```text
Product Identity
      +
Design Grammar
      ↓
screen generation
```

This creates a more unique result.

---

# 34. Design Tokens

Where appropriate maintain:

```text
colors

spacing

font scale

radius

shadow

animation duration

breakpoints
```

in reusable code.

Examples:

```text
CSS variables

Tailwind theme

design token JSON

theme module
```

depending on repository architecture.

---

# 35. Existing Design Detection

For existing applications, AgentCode should inspect:

```text
global styles

component library

fonts

tokens

CSS variables

Tailwind configuration

layout patterns

screenshots

existing components
```

before generating replacements.

---

# 36. Preserve Good Existing Design

Design Mode should not assume:

```text
AI-generated redesign is automatically better.
```

It should identify:

```text
existing strengths

existing brand identity

well-designed components
```

and preserve them when appropriate.

---

# 37. Anti-AI-Slop System

AgentCode should explicitly detect common low-quality AI UI patterns.

Signals include excessive use of:

```text
generic gradient backgrounds

gradient text

huge hero text

floating glass cards

three identical feature cards

random purple/blue color schemes

everything inside rounded containers

default Shadcn appearance

overused pill buttons

excessive shadows

generic startup illustrations

fake meaningless metrics

generic "AI-powered" copy

huge whitespace without purpose

identical landing-page structures

icon + title + description repetition
```

None is absolutely forbidden.

Repeated default use without product justification should lower design quality.

---

# 38. Anti-Slop Classification

Design critic may label:

```text
GENERIC_PATTERN

UNJUSTIFIED_DECORATION

TEMPLATE_LIKE

LOW_INFORMATION_DENSITY

VISUAL_INCONSISTENCY

BRAND_MISMATCH

OVERSTYLING

UNDERSTYLING
```

---

# 39. Uniqueness Is Not Randomness

AgentCode must not solve generic design by creating:

```text
weird layout

unusable navigation

random animation

novelty for novelty's sake.
```

Good uniqueness comes from:

```text
product-specific hierarchy

appropriate density

custom component composition

strong typography

careful spacing

purposeful motion

brand-specific visual motifs
```

---

# 40. Reference-Based Design

Users may provide:

```text
screenshot

website reference

existing product

image

moodboard

rough drawing
```

AgentCode should analyze:

```text
layout hierarchy

spacing

type hierarchy

density

component behavior

motion concept

navigation structure
```

rather than blindly copying pixels.

---

# 41. Reference Design Boundary

References are inspiration.

AgentCode should avoid direct copyrighted asset cloning.

Prefer:

```text
derive principles
```

rather than:

```text
duplicate proprietary design exactly.
```

---

# 42. Screenshot-to-Design Workflow

Possible:

```text
REFERENCE SCREENSHOT
        ↓
VISUAL ANALYSIS
        ↓
STRUCTURE EXTRACTION
        ↓
DESIGN PRINCIPLES
        ↓
AGENTCODE DESIGN GRAMMAR
        ↓
IMPLEMENTATION
```

---

# 43. Multiple Design Directions

For major greenfield interfaces, AgentCode may generate:

```text
Direction A

Direction B

Direction C
```

at design-brief/wireframe level.

The user may select one.

This should not require three complete implementations unless requested.

---

# 44. Autonomous Design Mode

Goal Mode may also permit:

```text
Choose the strongest design direction yourself.
```

Then AgentCode evaluates directions using:

```text
product fit

usability

uniqueness

implementation cost

accessibility

consistency
```

---

# 45. Layout Planning

Before detailed styling, establish:

```text
page regions

navigation

information hierarchy

responsive behavior

primary actions

secondary actions
```

This prevents decorative styling from hiding structural UX problems.

---

# 46. Component Reuse

Before generating new components:

```text
search repository
```

for existing suitable components.

Prefer:

```text
reuse

adapt

extend
```

over unnecessary duplication.

---

# 47. Component Quality

Components should be:

```text
reusable where appropriate

accessible

responsive

typed

consistent

testable
```

but AgentCode should avoid premature abstraction.

One unique component used once does not automatically need a framework-level abstraction.

---

# 48. Design Studio Edit Loop

```text
implement
   ↓
run application
   ↓
open browser
   ↓
capture screenshot
   ↓
inspect DOM
   ↓
inspect responsive state
   ↓
visual critic
   ↓
identify issues
   ↓
repair
```

This loop may run several times autonomously.

---

# 49. Deterministic Browser Validation

Use Playwright for:

```text
viewport size

element presence

layout dimensions

overflow

interactive flows

console errors

network failures
```

---

# 50. Visual Model Validation

Use visual models such as local Gemma3 4B for cheap checks:

```text
visual imbalance

clipping

strange spacing

missing component

obvious inconsistency
```

A stronger cloud visual model may be selected when design quality is important.

---

# 51. Visual Critic Independence

Where budget permits:

```text
UI Implementer
```

and:

```text
Visual Critic
```

should not always use the identical model.

Independent visual review can reduce self-confirmation.

---

# 52. Screenshot Baselines

Design tasks may store:

```text
before screenshot

after screenshot
```

for comparison.

Regression-sensitive repositories may maintain approved snapshots.

---

# 53. Responsive Testing

Minimum standard viewports should include representative:

```text
mobile

tablet

desktop

large desktop
```

Exact widths depend on project.

---

# 54. Responsive Validation

Check:

```text
horizontal overflow

navigation collapse

touch targets

text wrapping

modal fit

table behavior

form usability

image scaling

sidebar behavior
```

---

# 55. Accessibility

Design Studio must treat accessibility as part of quality.

Check where practical:

```text
semantic HTML

keyboard navigation

focus visibility

labels

ARIA

contrast

heading structure

touch-target size

reduced motion

screen-reader relationships
```

---

# 56. Accessibility Tools

AgentCode may integrate:

```text
Playwright accessibility checks

axe-core or equivalent

browser accessibility tree
```

through Doc 04.

Exact OSS extraction belongs to Doc 07.

---

# 57. Motion

Animation should be purposeful.

Acceptable purposes:

```text
state transition

spatial continuity

feedback

hierarchy

progress
```

Avoid:

```text
constant decorative movement

long intro animations

gratuitous floating objects
```

---

# 58. Motion Performance

Respect:

```text
prefers-reduced-motion
```

and avoid animations that significantly affect performance.

---

# 59. Typography

AgentCode should reason about:

```text
type family

weight hierarchy

line height

line length

density

technical readability
```

Typography often contributes more to perceived design quality than decorative effects.

---

# 60. Font Dependency Policy

Design Mode should prefer:

```text
existing project fonts
```

or:

```text
safe properly licensed web fonts
```

before introducing unnecessary font dependencies.

---

# 61. Color

Color should derive from:

```text
brand

product purpose

information hierarchy

semantic states
```

not default AI preference.

Semantic roles:

```text
background

surface

primary

secondary

success

warning

danger

muted
```

---

# 62. Dark Mode

If application supports dark/light appearance:

```text
both modes must be intentionally designed.
```

Dark mode should not simply invert colors.

---

# 63. Data-Dense Interfaces

Developer/admin/professional applications may need higher density than marketing pages.

AgentCode must not force:

```text
large empty cards
```

onto interfaces where users need:

```text
logs

tables

code

metrics

task state
```

---

# 64. Copy Quality

Design Studio should also critique UI copy.

Avoid generic filler such as:

```text
Unlock the power of...

Transform your workflow...

Take your experience to the next level...
```

Prefer product-specific language.

---

# 65. Real Data vs Fake Data

AgentCode should distinguish:

```text
real repository/application state
```

from:

```text
mock preview content.
```

Do not ship fake metrics as though they are real.

---

# 66. Loading States

Generated UI should consider:

```text
loading

empty

error

success

offline

disabled

permission-denied
```

not only the happy path.

---

# 67. Error Design

Error states must tell the user:

```text
what happened

whether AgentCode recovered

whether user action is needed
```

Example:

```text
NVIDIA became unavailable.
AgentCode continued using ModelScope.

No action required.
```

This is superior to:

```text
Provider Error 503.
```

---

# 68. AgentCode Desktop Navigation

The AgentCode desktop shell should remain small.

Recommended top-level navigation:

```text
Projects

Current Mission

Discuss

Design

Security
```

Secondary/advanced surfaces may live within project/mission detail.

---

# 69. Project List

Example:

```text
AgentCode

Projects

AstraFlow          Working
Smart Cricket      Idle
Move Party         Complete
AgentCode          Idle

[ Open Project ]
```

---

# 70. Project Opening

User may:

```text
choose folder

open recent repository

drag repository folder
```

AgentCode then starts Doc 02 repository bootstrap.

---

# 71. Repository Bootstrap UI

Keep simple:

```text
Analysing repository...

Languages detected:
TypeScript, Rust

12,482 files
Initial index ready
Deep analysis continuing
```

User may begin simple interaction before every optional index completes.

---

# 72. First-Time Project Summary

After bootstrap:

```text
Next.js frontend

Rust service

PostgreSQL database

84 tests

3 primary applications

Git branch: main
```

No huge automatically generated architecture report should block the screen.

Detailed repository understanding remains inspectable.

---

# 73. Mission Composer

Mission entry should support:

```text
multiline goal

attachments

screenshots

reference files

optional mode
```

Potential controls:

```text
Goal

Design

Security
```

but AgentCode may infer the appropriate execution type.

---

# 74. Mission Policies

Advanced users may expand:

```text
Mission Settings
```

to set:

```text
paid budget

autonomy level

parallelism

security depth

preferred model

network permissions
```

These should not clutter default mission creation.

---

# 75. Autonomy Profiles

Possible friendly profiles:

```text
Standard

High Autonomy

Restricted
```

Internally they map to Doc 04 permission policies.

---

# 76. Standard Profile

Routine local development:

```text
R0/R1 automatic

selected R2 automatic after checkpoint

external mutation requires permission
```

This should be the default.

---

# 77. High Autonomy

For trusted projects:

```text
routine local operations

dependency installs

approved staging operations

feature-branch push
```

may run automatically according to user configuration.

Production boundaries remain protected.

---

# 78. Restricted

Useful for unknown repositories:

```text
read-heavy

limited writes

network restrictions

more approvals
```

---

# 79. Security Mode UI

Security entry page may offer:

```text
Quick Audit

Full Security Audit

Cloud Audit

AI Security Audit

Adversarial Validation
```

The UI translates these into Doc 05 tasks/policies.

---

# 80. Security Scope Confirmation

Active testing should clearly display target.

Example:

```text
Target:
Local staging environment

Active validation:
Enabled

Production systems:
Not included
```

The user should always understand what will be attacked.

---

# 81. Security Findings UI

Default:

```text
2 High
4 Medium
7 Low
```

Each finding expands into:

```text
problem

evidence

attack path

fix

verification
```

---

# 82. Security Attack Path Visualization

A simple graph may be valuable.

Example:

```text
Public API
    ↓
Missing Ownership Check
    ↓
Object Key Exposure
    ↓
Private Storage Access
```

This is functional visualization, not decorative office UI.

---

# 83. Security Fix Action

Possible:

```text
[ Fix Confirmed Issues ]
```

This generates normal Kernel repair tasks.

---

# 84. Design Mode Entry

Example:

```text
Describe the interface you want.

[ Make this dashboard feel like a premium,
  distinctive professional VPN application.
  Preserve functionality but completely
  improve visual quality. ]

References:
+ Add Screenshot

[ Start Design Mission ]
```

---

# 85. Design Preview Layout

Design Mode may use:

```text
left:
brief / instructions

center:
live application preview

right:
optional design details
```

But AgentCode's normal Goal Mode should not become a permanent three-pane IDE.

---

# 86. Fullscreen Preview

Users should be able to maximize preview quickly.

Design decisions are easier when the application is shown at realistic size.

---

# 87. Direct Visual Selection

Future/high-value V1+ capability:

User clicks UI element in preview.

AgentCode resolves:

```text
DOM node
     ↓
React component
     ↓
source file
```

then the user can say:

```text
Make this header much cleaner.
```

Onlook is a useful reference for this interaction pattern.

---

# 88. Visual Selection Architecture

```text
browser DOM element
       ↓
source mapping
       ↓
component
       ↓
symbol/file
       ↓
Doc 02 context
       ↓
Worker edit
```

Where framework/source maps make this technically feasible.

---

# 89. Design History

Each significant design iteration should preserve:

```text
screenshot

change set

design brief version
```

User may compare prior versions.

---

# 90. Design Rollback

A rejected design iteration should be safely revertible through Doc 04 Git/change-set mechanisms.

---

# 91. Product Design Memory

AgentCode should remember project-specific design decisions.

Examples:

```text
navigation remains left rail

radius scale is small

primary typeface X

avoid gradients

dense technical layout

motion minimal
```

Store as structured project design knowledge.

---

# 92. `DESIGN_STATE.md`

Recommended readable snapshot:

```markdown
# AgentCode Design State

## Product Personality

...

## Target Users

...

## Design Principles

...

## Design Tokens

...

## Layout Rules

...

## Component Patterns

...

## Accessibility Constraints

...

## Responsive Rules

...

## Avoided Patterns

...

## Important Decisions

...

## Current Screens

...
```

---

# 93. Design State Authority

`DESIGN_STATE.md` is a readable handoff.

Actual:

```text
source code

design tokens

components

browser output
```

remain authoritative.

---

# 94. Design Knowledge Freshness

If design-system code changes:

```text
stored design facts
```

must be revalidated under Doc 02 freshness rules.

---

# 95. Design Skills

Suggested skills:

```text
ui-design

ux-design

design-director

frontend-implementation

responsive-design

accessibility

motion-design

design-system

visual-qa

anti-ai-slop
```

Skills load only when needed.

---

# 96. Design Context Pack

A design task should receive:

```text
product goal

design brief

existing design state

screenshots

relevant components

design tokens

current page

related routes

brand assets

reference analysis

functional requirements
```

Do not send entire unrelated backend implementation.

---

# 97. Design Token Efficiency

For design work:

```text
visual screenshot
+
DOM/accessibility tree
+
relevant component source
+
design system
```

may be much more valuable than:

```text
200K source tokens.
```

Doc 02 relevance engine should construct design-specific context.

---

# 98. Design Image Handling

Screenshots and reference imagery should be treated as context artifacts.

Store:

```text
artifact ID

viewport

page

commit

task
```

---

# 99. Design Studio Browser Loop

```text
Worker edits UI
       ↓
run dev server
       ↓
Playwright opens page
       ↓
capture DOM + screenshot
       ↓
Visual QA
       ↓
functional QA
       ↓
design critic
       ↓
repair
```

No user babysitting should be required for ordinary iterations.

---

# 100. Visual Regression Detection

Where a repository has existing baselines:

```text
compare before / after
```

and detect unexpected changes.

Not every pixel difference is failure.

Model/DOM reasoning should differentiate intended redesign from accidental regression.

---

# 101. Design Quality Dimensions

AgentCode should assess:

```text
product fit

visual hierarchy

layout quality

typography

spacing

consistency

interaction clarity

responsiveness

accessibility

uniqueness

information density

performance

implementation quality
```

---

# 102. Design Score Is Advisory

If internal design scores are used:

```text
84 / 100
```

they must not become fake scientific truth.

Scores can guide iteration.

Acceptance should depend on concrete checks and product goals.

---

# 103. Anti-Slop Review Questions

Before accepting major generated UI, ask:

```text
Does this resemble a generic AI SaaS template?

Could this design belong to almost any product?

Does the design communicate the product's actual purpose?

Are default components visibly uncustomized?

Is the information hierarchy deliberate?

Is there unnecessary gradient/glass/card usage?

Does the typography feel intentional?

Is whitespace functional?

Are important workflows immediately clear?

Does it still look good without decorative effects?
```

---

# 104. Existing Product Consistency

When adding a new screen:

```text
do not create an entirely new visual language
```

unless the mission explicitly calls for a redesign.

Match existing:

```text
spacing

components

navigation

type hierarchy

interaction behavior
```

---

# 105. Greenfield Design

For new projects AgentCode should establish design foundations early:

```text
tokens

layout system

navigation

type scale

component primitives

responsive system
```

before generating dozens of unrelated pages.

---

# 106. Design System Growth

Do not generate:

```text
100 components
```

before they are needed.

Start with a small coherent primitive set.

Expand as screens demand.

---

# 107. Preview Data

Preview environments may use:

```text
synthetic test data
```

clearly separated from real production data.

---

# 108. Backend Integration

Design Mode must not produce fake nonfunctional UI when real backend contracts exist.

It should use Doc 02 API intelligence to wire:

```text
actual data

actual endpoints

actual states
```

when task requires functioning implementation.

---

# 109. Design-Only vs Full Implementation

Two task classes:

```text
DESIGN_ONLY

DESIGN_AND_IMPLEMENT
```

Design-only may create:

```text
brief

wireframe

prototype
```

without changing production code.

Design-and-implement modifies repository and undergoes normal verification.

---

# 110. Functional Verification After Design

A beautiful design that breaks functionality is failure.

Run:

```text
existing tests

browser flows

forms

navigation

API interaction

responsive checks
```

after UI redesign.

---

# 111. Performance

Design quality includes application performance.

Avoid:

```text
huge image assets

unnecessary animation libraries

massive dependencies

excessive client JS

unoptimized video
```

unless justified.

---

# 112. UX State Coverage

Generated components should account for:

```text
normal

hover

focus

active

disabled

loading

error

empty

success
```

where applicable.

---

# 113. Completion Notifications

AgentCode should notify on meaningful events.

Primary:

```text
MISSION COMPLETE
```

Secondary:

```text
NEEDS USER
```

Optional:

```text
MISSION FAILED/BLOCKED
```

Do not notify for routine progress.

---

# 114. Completion Notification Example

```text
AgentCode

Mission complete

14/14 requirements verified
318 tests passed
0 blocking security findings
```

Click opens mission summary.

---

# 115. Completion Sound

Successful mission should produce:

```text
subtle
pleasant
short
smooth
```

sound.

Target duration:

```text
approximately 0.5–1.0 seconds
```

It should not resemble:

```text
alarm

game achievement

loud bell
```

---

# 116. Needs-You Sound

Human-intervention notification may use a slightly distinct but still minimal sound.

Completion and attention sounds should be distinguishable without being distracting.

---

# 117. Sound Controls

Settings:

```text
Completion sounds:
On / Off

Attention sounds:
On / Off

Volume:
respect system volume
```

No custom sound-management complexity required in early V1.

---

# 118. Do Not Spam Notifications

Examples that should not trigger user notification:

```text
provider switched

worker restarted

test failed during normal iteration

context compacted

task started
```

if AgentCode can handle them autonomously.

---

# 119. Background Runtime UX

Closing the UI should display once, when relevant:

```text
AgentCode will continue working in the background.
```

Do not show repeatedly.

---

# 120. Menu Bar / Tray

A small system tray/menu-bar presence may expose:

```text
Current mission

Pause

Resume

Open AgentCode

Quit
```

This is useful for background execution.

---

# 121. Quit Behavior

If a mission is active and user chooses full daemon quit:

```text
checkpoint active work
```

then confirm whether to stop background execution.

Closing a window is different from terminating the daemon.

---

# 122. Pause UX

Pause:

```text
AgentCode is pausing safely...
```

then:

```text
Paused
```

Do not abruptly terminate a file transaction.

---

# 123. Resume UX

Resume should normally require one action.

AgentCode handles internal reconciliation automatically.

---

# 124. Error Communication

Error messages should prioritize:

```text
impact

recovery

required user action
```

Example:

```text
GitHub authentication expired.

AgentCode paused the PR publishing task.
All local work is preserved.

[Reconnect GitHub]
```

---

# 125. Recovery Communication

Automatic recovery should be concise.

Example:

```text
Recovered from provider failure.
```

Detailed logs remain available.

---

# 126. Project Search

As projects accumulate, support:

```text
search by name

recent projects

status
```

No complex workspace hierarchy required initially.

---

# 127. Global Command Palette

Useful actions:

```text
Open Project

Start Mission

Discuss

Design

Security Audit

Pause Mission

View Changes
```

A minimal command palette improves expert workflow without cluttering the UI.

---

# 128. Keyboard Shortcuts

V1 should support a small useful set.

Potential:

```text
Cmd/Ctrl + K
Command Palette

Cmd/Ctrl + Enter
Submit goal/message

Cmd/Ctrl + Shift + P
Project switch
```

Avoid dozens of obscure shortcuts initially.

---

# 129. Search Within Mission

Advanced users should be able to find:

```text
task

file

security finding

event

decision
```

inside a long mission history.

---

# 130. Human Review After Completion

Completion screen:

```text
Mission Complete

Requirements     14/14

Tests            318 passed

Security         No blocking findings

Files Changed    17

Duration         2h 41m

[Review Changes]
[View Report]
[Continue Discussion]
```

---

# 131. Continue After Completion

The project retains context.

User may say:

```text
Now add Google OAuth.
```

A new mission starts with existing project intelligence and decisions.

---

# 132. Project Timeline

Project timeline may include:

```text
missions

major decisions

security audits

design changes
```

without retaining every model token in the main UI.

---

# 133. User Trust

AgentCode should make autonomy understandable.

The user must always be able to answer:

```text
What changed?

Why?

What was tested?

Which requirements passed?

What remains risky?
```

Transparency must exist without forcing constant observation.

---

# 134. No Fake AI Mysticism

Avoid UI labels like:

```text
AI is thinking deeply...

Our agents are collaborating...

Neural swarm activated...
```

Prefer concrete engineering states:

```text
Inspecting authentication tests

Running build

Verifying session flow
```

---

# 135. No Unnecessary Anthropomorphism

Use:

```text
Worker

Verifier

Planner
```

primarily in advanced detail.

Normal UI may say:

```text
Implementing

Verifying

Researching
```

instead of constantly describing personalities.

---

# 136. Theme

AgentCode should support:

```text
System

Light

Dark
```

with a high-quality dark mode because developer tools often operate in dark environments.

Theme should follow native system setting by default.

---

# 137. Accent Color

Use restrained accent color.

Functional semantic colors remain:

```text
success

warning

error

active
```

Avoid turning the entire application into a saturated gradient-heavy interface.

---

# 138. Tauri Desktop Shell

Recommended V1 desktop shell:

```text
Tauri 2
+
React
+
TypeScript
+
Vite
```

Reasons:

```text
lower runtime overhead than Electron

native desktop packaging

good web UI ecosystem

suitable for target 8 GB Mac

fits minimal AgentCode interface
```

This should be validated during Doc 07 extraction before final implementation commitment if a serious compatibility issue emerges.

---

# 139. Editor Components

For source/diff inspection:

```text
Monaco
```

is a strong V1 candidate.

AgentCode is not intended to reproduce every feature of VS Code.

The embedded editor serves:

```text
review

small manual edits

diff inspection
```

---

# 140. Terminal Component

Use an xterm-compatible terminal implementation such as:

```text
xterm.js
```

when interactive terminal display is required.

Again:

```text
terminal is a capability,
not the center of the UI.
```

---

# 141. Native Menus

Desktop application should expose standard:

```text
File

Edit

View

Window
```

behavior where appropriate.

Avoid unnecessarily custom replacement for every operating-system convention.

---

# 142. Accessibility of AgentCode Itself

AgentCode UI must support:

```text
keyboard navigation

screen readers

visible focus

reasonable contrast

reduced motion

semantic labels
```

AgentCode should satisfy the quality standard it expects from generated applications.

---

# 143. Performance UX

The interface must remain responsive while:

```text
indexing repository

running missions

streaming logs

browser testing
```

Heavy work belongs in daemon/background processes.

---

# 144. UI State Isolation

A UI crash must not corrupt Kernel state.

The UI reads:

```text
mission state
```

through a stable local API/event stream.

---

# 145. Event Streaming

Daemon can emit compact events such as:

```text
TaskUpdated

MissionProgressChanged

FindingCreated

UserActionRequired

MissionCompleted
```

UI subscribes and updates.

---

# 146. Avoid Streaming Every Token

The interface does not need every LLM token from every worker.

Default mission UI receives:

```text
structured progress events
```

rather than huge raw model streams.

This reduces CPU/UI noise and preserves minimalism.

---

# 147. Optional Live Model Output

Advanced debugging may expose current model output.

Hidden by default.

---

# 148. Activity Compression

Repeated events may collapse.

Instead of:

```text
Read file A
Read file B
Read file C
Read file D
```

show:

```text
Inspected 14 authentication files
```

Expandable to raw events.

---

# 149. Cost Display

Default mission UI does not need constant token counters.

Mission detail may show:

```text
Free provider usage

Paid usage

₹3.42 spent
```

if relevant.

---

# 150. Paid Budget Warning

If mission approaches configured paid limit:

```text
Paid reserve nearly exhausted.
```

Only interrupt if further execution requires additional authorization.

---

# 151. Context UI

Advanced Context panel may show:

```text
Context Pack

31.4K tokens

14 files

38 symbols

7 tests

2 decisions
```

Each entry can explain why it was included.

This consumes Doc 02 provenance data.

---

# 152. Model Routing UI

Advanced panel:

```text
Task T42

Coder:
Qwen / ModelScope

Verifier:
GPT-OSS / Cerebras

Why:
coding specialization
provider diversity
quota available
```

Again, hidden from normal use.

---

# 153. Worktree UI

Normal UI:

```text
3 tasks running
```

Advanced:

```text
T42 → worktree ...
T43 → worktree ...
```

Do not expose worktree complexity unless useful.

---

# 154. Project Rules UI

Advanced project settings may manage:

```text
AgentCode rules

scoped instructions

trusted skills

providers

security permissions
```

---

# 155. Design Studio References

Primary cloned references:

```text
onlook

dyad

bolt.diy
```

---

# 156. Onlook

Study for:

```text
visual editing

DOM/source mapping

React component awareness

preview-to-code workflows

direct manipulation ideas
```

AgentCode should adapt concepts, not copy product identity.

---

# 157. Dyad

Study for:

```text
local prompt-to-app workflow

preview

iterative generation

BYOK model architecture

app-builder simplicity
```

License boundaries in `src/pro` must be handled carefully in Doc 07.

---

# 158. Bolt.diy

Study for:

```text
prompt-to-app workflow

live execution

code generation

preview

provider flexibility
```

WebContainer/licensing boundaries must be reviewed in Doc 07.

---

# 159. Codex

Study for:

```text
minimal task UI

mission/task presentation

diff review

background coding workflow

low visual noise
```

---

# 160. OpenCode / Cline / Goose

Study for:

```text
desktop coding UX

tool activity

session management

terminal visibility

approvals

task history
```

Do not copy their layouts blindly.

---

# 161. Browser Use / Playwright

Use for:

```text
preview control

functional QA

screenshot loop

responsive testing
```

---

# 162. Anti-Slop Skill Sources

Useful patterns may also come from:

```text
design-focused skill repositories

existing design-system best practices

Onlook

human-curated UI guidance
```

But AgentCode should own the final Design Director / Critic behavior.

---

# 163. What AgentCode Should Reuse

Reuse proven primitives for:

```text
browser automation

code editing

source mapping

rendering

desktop framework

terminal

diff editor
```

where licenses and architecture allow.

---

# 164. What AgentCode Must Own

AgentCode must own:

```text
minimal mission UX

Design Studio workflow

design grammar

anti-AI-slop rules

project design memory

visual critique orchestration

notification philosophy

progressive disclosure

background mission experience
```

These define AgentCode's product identity.

---

# 165. What We Must Not Do

Do not create:

```text
office simulation

agent avatars

huge always-visible dashboards

provider-centric UI

terminal-first UI

chat-only UI

generic AI SaaS styling

gradient-heavy default design

three-card-template everywhere

fake progress percentages

fake metrics

fake agent activity

constant notifications

approval prompts for safe routine actions

full raw logs by default

every model token streaming into UI
```

---


# HARDENING REVISION 2 — IMPLEMENTATION-GRADE PRODUCT UX & DESIGN STUDIO CONTRACTS

This hardening section is normative. It preserves the architecture and product philosophy defined in Sections 1–165, but replaces any ambiguity left by conceptual examples with implementation-grade contracts.

If an older section says “possible,” “recommended,” “suggested,” or provides only an illustrative mockup, the following rules determine what is architectural truth, what remains an implementation choice, what is benchmark-tunable, and what is optional.

The purpose of this revision is not to make AgentCode visually larger or more complicated. It is to make the implementation contract **more exact while keeping the product itself minimal**.

The governing UX promise remains:

```text
OPEN PROJECT
    ↓
DESCRIBE GOAL
    ↓
START
    ↓
LEAVE
    ↓
AGENTCODE WORKS AND RECOVERS
    ↓
AGENTCODE VERIFIES
    ↓
NOTIFY ONLY WHEN MEANINGFUL
    ↓
REVIEW EVIDENCE
```

The Design Studio promise remains:

```text
UNDERSTAND PRODUCT
    ↓
ESTABLISH DESIGN INTENT
    ↓
IMPLEMENT
    ↓
RUN REAL APPLICATION
    ↓
INSPECT FUNCTION + VISUALS
    ↓
CRITIQUE
    ↓
REPAIR
    ↓
VERIFY
```

Neither workflow may depend on the user babysitting normal local development operations.

---

## H1. Normative Decision Classes

Every decision in this document belongs to one of the following classes.

| Class | Meaning | Examples in Doc 06 |
|---|---|---|
| `LOCKED_ARCHITECTURE` | Product identity or subsystem ownership that must not change without an ADR and cross-document reconciliation. | Four top-level modes; Goal Mode flagship; UI does not own mission truth; no office/avatar metaphor; progressive disclosure; background daemon survives window closure; Design Studio uses real browser feedback; Security UI exposes scope. |
| `CONSTRAINED_IMPLEMENTATION_DECISION` | More than one implementation is acceptable, but it must satisfy the contract defined here. | Exact desktop route library; state-management library; local IPC transport; exact component library; exact screenshot storage encoding. |
| `BENCHMARK_PENDING` | Initial behavior is defined, but thresholds must be calibrated against fixtures and the target 8 GB Mac. | UI memory budget, screenshot retention count, visual-diff thresholds, browser concurrency, activity-collapse thresholds. |
| `DYNAMIC_RUNTIME_DATA` | Runtime state that must never be hardcoded into product architecture. | Current model/provider names, task count, current context size, current scanner availability, current repository languages. |
| `REQUIRED_V1` | Capability required for the minimum V1 product to satisfy this document. | Goal Mode, mission status, Changes, Activity, Discuss, background execution, notifications, core Design Studio pipeline, accessibility of AgentCode itself. |
| `REQUIRED_IF_APPLICABLE` | Required when the project/task exposes the relevant surface. | Mobile viewport validation for mobile web apps; Security scope UI for active validation; dark/light parity when both themes are supported. |
| `OPTIONAL_V1` | Useful capability that may ship in V1 if quality gates pass but is not necessary for core release. | Direct visual element selection/source mapping; multi-direction design comparison UI; advanced visual baseline management. |
| `POST_V1` | Deliberately outside core V1. | Team collaboration UI, remote multi-user mission dashboards, marketplace-like design asset ecosystem, full visual IDE replacement. |

These classes resolve a previous ambiguity: **Design Studio is a first-class product mode, but every advanced design interaction does not need to be release-critical.** V1 must ship a serious prompt-to-working-interface pipeline with real browser feedback; direct DOM-to-source visual editing may remain optional until it is trustworthy.

---

## H2. Canonical Product Surface and Route Hierarchy

The desktop information architecture must be stable enough that internal implementation agents do not invent a different navigation model for each feature.

The logical route hierarchy is:

```text
/
├── projects
│   ├── recent
│   └── project/:project_id
│       ├── overview
│       ├── goal
│       ├── discuss
│       ├── design
│       ├── security
│       ├── mission/:mission_id
│       │   ├── summary
│       │   ├── changes
│       │   ├── activity
│       │   ├── verification
│       │   ├── security
│       │   └── details
│       └── settings
└── settings
    ├── general
    ├── providers
    ├── permissions
    ├── appearance
    └── diagnostics
```

The concrete routing library is a `CONSTRAINED_IMPLEMENTATION_DECISION`, but the information architecture is `LOCKED_ARCHITECTURE`.

### Default route behavior

Opening AgentCode should resolve in this order:

1. if an active mission was open when the UI closed, restore that mission summary;
2. otherwise, if a project was active recently, restore the project overview;
3. otherwise show Projects.

The UI must never fabricate a mission from local visual state. It requests the authoritative snapshot from the daemon.

### Back/forward semantics

Route navigation must be normal desktop/web navigation, not a custom modal stack for every surface. Closing a detail panel returns to the prior logical surface without losing daemon-backed state.

---

## H3. UI Ownership and Non-Ownership Matrix

The UI is a **projection of authoritative subsystem state**, not a second control plane.

| Surface | UI owns | UI must never own |
|---|---|---|
| Project picker | folder selection intent, recent-project presentation | repository identity, index truth |
| Goal composer | unsent draft text, attachments before mission creation | mission state, requirement state |
| Mission screen | view filters, expanded/collapsed panels | task lifecycle, progress truth, completion |
| Discuss | local draft, conversation viewport state | repository truth, accepted decisions until promoted |
| Design Studio | visual workspace layout, selection state | code truth, browser truth, design acceptance |
| Security | presentation filters, user-entered scope proposal | authorization truth, finding truth, scanner execution |
| Changes | diff selection, display preferences | Git/worktree truth |
| Activity | grouping/presentation | event truth |
| Settings | user configuration intent | secret material, provider health truth |
| Notifications | presentation | deciding whether mission is complete |
| Terminal | terminal viewport | process ownership/lifecycle |

The UI may issue commands such as:

```text
StartMission
PauseMission
ResumeMission
CancelMission
RespondToHumanRequest
CreateDiscussion
PromoteDecision
StartDesignMission
StartSecurityAudit
```

but only the daemon/Kernel determines whether the command is valid and persists the resulting transition.

---

## H4. Canonical UI ↔ Daemon Contract

The exact local transport is defined outside this document, but the logical protocol is required.

### Request envelope

```ts
type UiRequest<T> = {
  protocol_version: string
  request_id: string
  operation_id?: string
  client_id: string
  sent_at: string
  command: string
  payload: T
}
```

### Response envelope

```ts
type UiResponse<T> = {
  protocol_version: string
  request_id: string
  status: "OK" | "REJECTED" | "CONFLICT" | "UNAVAILABLE" | "ERROR"
  result?: T
  error?: UiError
  server_revision: number
}
```

### Event envelope

```ts
type UiEvent<T> = {
  protocol_version: string
  event_id: string
  stream_revision: number
  event_type: string
  project_id?: string
  mission_id?: string
  correlation_id?: string
  created_at: string
  payload: T
}
```

### Required event families

At minimum the desktop consumes:

```text
DaemonReady
DaemonDegraded
ProjectOpened
ProjectIndexStateChanged

MissionCreated
MissionStateChanged
MissionProgressChanged
MissionBlocked
MissionNeedsUser
MissionCompleted
MissionCancelled

TaskSummaryChanged
VerificationSummaryChanged
SecuritySummaryChanged
ChangesSummaryChanged

ActivityEventCreated
HumanRequestOpened
HumanRequestResolved

ProviderRecoverySummary
BudgetWarning

DesignPreviewReady
DesignIterationRecorded
DesignFindingChanged

SecurityFindingChanged
AttackPathChanged
```

The UI should **not** subscribe to every raw model token or tool stdout line by default.

---

## H5. Snapshot-Then-Stream Reconnection

Renderer restart, Tauri window recreation, laptop sleep, daemon restart, or temporary IPC loss must not leave the interface in a half-true state.

Reconnect algorithm:

```text
UI reconnects
    ↓
protocol handshake
    ↓
request authoritative snapshot
    ↓
receive snapshot_revision = N
    ↓
subscribe to events after N
    ↓
apply monotonic event stream
```

If a revision gap is detected:

```text
stop incremental apply
→ request fresh snapshot
→ resume stream
```

The renderer may maintain optimistic state only for unsent drafts or immediately reversible local UI actions. Mission state itself is never optimistically invented.

### Visible reconnect states

Short reconnects should normally show a subtle indicator:

```text
Reconnecting…
```

If the daemon remains unavailable:

```text
AgentCode background service is unavailable.
Your repository has not been modified by this UI error.

[Retry] [Diagnostics]
```

No mission should be labeled failed merely because the renderer cannot currently reach the daemon.

---

## H6. Canonical Desktop View-State Model

The desktop should separate four classes of state.

### 1. Authoritative remote-local state

From daemon:

```text
projects
missions
requirements summary
task summaries
verification
security findings
events
budgets
human requests
```

### 2. Persisted user preferences

Examples:

```text
theme
accent
sound preferences
last project
panel widths
preferred detail density
terminal font size
```

### 3. Ephemeral view state

Examples:

```text
selected tab
expanded finding
current diff file
scroll location
open command palette
```

### 4. Draft state

Examples:

```text
goal draft
discussion draft
design instruction draft
security scope form before submit
```

A renderer crash may lose some ephemeral state. It must not lose authoritative project/mission state.

---

## H7. Mission Status Projection

The user-facing states in Section 12 are **projections**, not new Kernel states.

Canonical mapping concept:

| Kernel condition | Default user-facing label |
|---|---|
| repository bootstrap not minimally ready | `Analysing` |
| mission interpreting/decomposing | `Planning` |
| runnable implementation work active | `Working` |
| mechanical/runtime checks dominate current critical path | `Testing` |
| independent verification active | `Verifying` |
| security verification dominates current phase | `Security Audit` |
| mission final audit | `Final Audit` |
| blocking human request open | `Needs You` |
| mission paused | `Paused` |
| unresolved external/system blocker | `Blocked` |
| committed complete | `Complete` |
| recovery in progress and user action not needed | retain current label + subtle `Recovering` detail |

The UI should not flicker between labels because a five-second test run begins. A **dominant phase debounce/hysteresis** should prevent visual thrashing.

---

## H8. Truthful Progress Computation

A numeric percentage is allowed only when the Kernel exposes enough stable information to compute one without misleading the user.

Initial V1 algorithm:

```text
mission_progress =
    weighted_requirement_progress × 0.50
  + weighted_task_progress        × 0.25
  + verification_progress         × 0.15
  + integration_progress          × 0.05
  + finalization_progress         × 0.05
```

This is an initial `BENCHMARK_PENDING` profile, not immutable mathematics.

### Requirement weighting

Requirements should dominate because completing twenty incidental tasks should not imply that the actual user goal is nearly finished.

Each blocking requirement receives a weight derived from:

```text
risk
scope
critical-path relevance
verification burden
```

### Task contribution

A task contributes no more than its requirement coverage permits.

Recommended task progression:

```text
READY            0.00
RUNNING          0.20
IMPLEMENTED      0.45
VERIFYING        0.60
REPAIR           0.45
PASSED           0.80
INTEGRATING      0.88
INTEGRATED       0.94
COMPLETE         1.00
```

These are UI projection weights, **not Kernel state semantics**.

### Anti-regression rule

Progress may legitimately decrease after:

```text
new requirement discovered
verification invalidated
integration regression
security finding blocks completion
```

If it decreases materially, the UI should explain why rather than silently animate backward.

### When to hide percentage

Hide numeric progress when:

- the plan is not yet sufficiently stable;
- requirement extraction is materially incomplete;
- a large replan invalidated weight assumptions;
- critical work remains unbounded.

Show:

```text
Working
```

with milestone summaries instead.

---

## H9. Activity Compression Contract

Activity must communicate meaningful engineering progress without turning into a token/tool-call firehose.

Raw events remain available, but default activity uses grouped `ActivityItem` records.

```ts
type ActivityItem = {
  activity_id: string
  mission_id: string
  category:
    | "ANALYSIS"
    | "IMPLEMENTATION"
    | "TEST"
    | "VERIFICATION"
    | "SECURITY"
    | "RECOVERY"
    | "HUMAN"
    | "SYSTEM"
  title: string
  detail?: string
  started_at: string
  ended_at?: string
  status: "ACTIVE" | "PASS" | "WARN" | "FAIL" | "INFO"
  source_event_ids: string[]
  expandable: boolean
}
```

### Collapse rules

Events may collapse when they are:

- same category;
- same task;
- close in time;
- non-error;
- not individually user-actionable.

Example:

```text
Read 14 files
Ran 6 code searches
Resolved 18 symbol references
```

may become:

```text
Inspected authentication implementation
```

Errors, human requests, verification failures, security findings, and significant recoveries are not silently collapsed away.

---

## H10. Mission Summary Contract

The completion screen should be built from a structured summary, not generated prose alone.

```ts
type MissionSummary = {
  mission_id: string
  goal: string
  final_state: "COMPLETE" | "CANCELLED" | "FAILED"
  requirement_counts: {
    total: number
    verified: number
    accepted_risk: number
    approved_out_of_scope: number
  }
  task_counts: {
    total: number
    completed: number
  }
  files_changed: number
  commits_created: number
  tests: {
    passed: number
    failed: number
    skipped: number
  }
  build_state?: string
  security: {
    blocking_open: number
    accepted_risks: number
  }
  duration_ms: number
  paid_cost_minor_units?: number
  final_audit_ref?: string
  known_limitations: string[]
}
```

The prose summary may explain the result, but counts must come from structured evidence.

---

## H11. First-Run and Onboarding Experience

AgentCode should not use a long tutorial carousel.

The first run should complete only the minimum setup necessary to make the first mission successful.

### First-run sequence

```text
Launch
  ↓
Choose/Open Repository
  ↓
Background daemon health check
  ↓
Optional provider/account setup if no usable route exists
  ↓
Repository bootstrap
  ↓
Goal composer
```

### Progressive setup

If local/offline capability exists, opening a repository must not be blocked because an optional cloud provider is not configured.

If no inference route is available, show one focused blocker:

```text
AgentCode needs at least one usable model route.

[Configure Provider]
[Use Local Model]
```

Do not expose the entire provider architecture during onboarding.

### Trust setup for unknown repositories

If the repository is untrusted, the first project screen may state:

```text
Restricted project permissions are active.
Network and high-risk operations require explicit approval.

[Review Permissions]
```

The UI should explain the security consequence without forcing the user through every internal risk class.

---

## H12. Project Bootstrap UX State Machine

The project-opening experience should reflect Doc 02 readiness rather than one generic spinner.

```text
OPENING
→ DISCOVERING
→ BASE_INDEX_READY
→ STRUCTURAL_READY
→ SEMANTIC_READY
→ KNOWLEDGE_READY
```

The UI may expose a compact state such as:

```text
Repository ready
Deep analysis continuing
```

when `BASE_INDEX_READY` or `STRUCTURAL_READY` is enough for the requested interaction.

### Degraded state

If LSP fails:

```text
Repository ready with reduced semantic intelligence.
Exact and structural search remain available.
```

This should normally be advanced-detail information unless it materially reduces task quality.

---

## H13. Goal Composer Contract

A `GoalDraft` contains:

```ts
type GoalDraft = {
  project_id: string
  text: string
  attachment_refs: string[]
  requested_mode?: "GOAL" | "DESIGN" | "SECURITY"
  mission_policy_overrides?: Partial<MissionPolicy>
}
```

### Submission validation

Block submission only for:

- empty goal;
- project unavailable;
- attachment still unresolved in a required way;
- no feasible model route and no usable local fallback;
- explicit policy conflict that cannot be resolved after submit.

Do **not** require the user to manually supply:

```text
task decomposition
test commands
model
provider
worktree count
context size
```

unless the user deliberately opens advanced settings.

---

## H14. Mode Semantics and Cross-Mode Transitions

The four top-level modes are locked:

```text
GOAL | DISCUSS | DESIGN | SECURITY
```

They share the same project intelligence and daemon.

### GOAL

Creates or operates on autonomous missions.

### DISCUSS

Read-only by default. May promote selected decisions into durable state.

### DESIGN

May be:

```text
DESIGN_ONLY
DESIGN_AND_IMPLEMENT
```

### SECURITY

May create passive or authorized active verification missions.

### Allowed promotion flows

```text
DISCUSS → DECISION
DISCUSS → PLAN
DISCUSS → GOAL MISSION

DISCUSS → DESIGN MISSION
DISCUSS → SECURITY MISSION

DESIGN_ONLY → DESIGN_AND_IMPLEMENT
SECURITY FINDING → REPAIR GOAL/TASK
```

No mode switch should require copying chat manually.

---

## H15. Discuss Session Contract

A Discuss session should be grounded but lighter than a mission.

```ts
type DiscussSession = {
  discuss_id: string
  project_id: string
  created_at: string
  updated_at: string
  read_only: boolean
  message_refs: string[]
  decision_candidates: string[]
  promoted_decision_refs: string[]
  generated_plan_ref?: string
  context_manifest_refs: string[]
}
```

### Read-only enforcement

Discuss may use:

```text
read/search
code intelligence
web/docs research where permitted
non-mutating diagnostics
```

but write tools are absent by default.

A user saying:

```text
fix it
```

should trigger an explicit transition:

```text
Create Mission
```

rather than silently mutating the repository under a conversational context.

---

## H16. Decision Promotion

Not every discussion sentence becomes project memory.

A `DecisionCandidate` should contain:

```ts
type DecisionCandidate = {
  candidate_id: string
  statement: string
  rationale?: string
  constraints: string[]
  rejected_alternatives?: string[]
  source_message_refs: string[]
  confidence: "EXPLICIT_USER_ACCEPTANCE" | "INFERRED_CANDIDATE"
}
```

Only accepted decisions become durable authoritative project decisions.

The UI should offer:

```text
Save Decision
Turn Into Plan
Execute
```

where relevant.

---

## H17. Security UX Contract

The Security UI is a presentation layer over Doc 05 policy and evidence.

### Security launch form

For passive audit:

```text
Audit depth
Repository scope
Optional cloud scope
```

For active validation:

```text
Environment classification
Exact target
Allowed domains/IPs/accounts
Credential profile
Rate/concurrency limit
Forbidden actions
Authorization expiry
```

### Safety clarity

The UI must make the active boundary visible before start.

Bad:

```text
Start Advanced Scan
```

Good:

```text
Active Validation
Target: staging.example.com
Environment: STAGING
Out-of-scope redirects: blocked
Destructive actions: prohibited
Rate limit: 5 req/s
```

### Production warning

For `PRODUCTION_ACTIVE_APPROVED`, the UI must use an explicit confirmation surface because the action class materially changes, but it should not repeatedly ask for every safe request once the bounded authorization is committed.

---

## H18. Security Finding Presentation

A security finding card must distinguish:

```text
severity
confidence
proof level
status
```

These are not interchangeable.

Example:

```text
HIGH severity
HIGH confidence
REACHABLE proof
OPEN
```

rather than one vague red badge.

The expanded view should show:

```text
What is wrong
Why it matters
Affected surface
Evidence
Attack path
Validation status
Recommended repair
Regression status
Accepted-risk/suppression state
```

Scanner names belong in evidence/provenance, not the headline.

---

## H19. Design Studio Canonical Task Types

Design Studio supports:

### `DESIGN_ONLY`

Produces design artifacts without production repository mutation.

Typical outputs:

```text
DesignBrief
DesignGrammar
wireframes
reference analysis
screen specification
prototype artifact
```

### `DESIGN_AND_IMPLEMENT`

Produces actual repository changes and enters normal edit/verification flow.

### `VISUAL_REPAIR`

Repairs a concrete visual/accessibility/responsive defect while preserving broader design intent.

### `DESIGN_SYSTEM_EVOLUTION`

Changes shared tokens/primitives/components. Because blast radius is larger, it requires broader visual and functional verification.

Task type is recorded so the Context Engine and Verification Engine can select appropriate profiles.

---

## H20. Canonical `DesignBrief`

The previous example becomes a formal record.

```ts
type DesignBrief = {
  design_brief_id: string
  project_id: string
  mission_id?: string
  version: number
  status: "DRAFT" | "APPROVED" | "ACTIVE" | "SUPERSEDED"

  product: {
    description: string
    category?: string
    maturity: "GREENFIELD" | "EXISTING"
  }

  users: Array<{
    segment: string
    primary_jobs: string[]
    expertise?: string
  }>

  primary_workflows: string[]
  primary_actions: string[]
  secondary_actions: string[]

  desired_personality: string[]
  undesired_personality: string[]

  information_density:
    | "LOW"
    | "MEDIUM"
    | "MEDIUM_HIGH"
    | "HIGH"

  accessibility_target: string
  target_form_factors: string[]

  preserve: string[]
  change: string[]
  do_not_touch: string[]

  reference_analysis_refs: string[]
  functional_requirement_refs: string[]
  brand_asset_refs: string[]

  quality_criteria: DesignQualityCriterion[]
  assumptions: string[]
  unresolved_questions: string[]

  provenance: {
    user_instruction_refs: string[]
    repository_evidence_refs: string[]
    generated_by_session?: string
  }

  created_at: string
  updated_at: string
}
```

### Brief rules

1. Product function must be represented before visual personality.
2. `do_not_touch` has higher design priority than aesthetic preference.
3. Functional requirements are not rewritten by the Design Director.
4. An unresolved question only interrupts the user if it materially changes product behavior or creates a high-cost rework risk.
5. The brief may be revised, but versions remain traceable.

---

## H21. Brief-Derived Quality Criteria

The design critic should not evaluate against generic taste only.

Each substantial brief produces criteria such as:

```ts
type DesignQualityCriterion = {
  criterion_id: string
  description: string
  category:
    | "PRODUCT_FIT"
    | "HIERARCHY"
    | "USABILITY"
    | "VISUAL"
    | "RESPONSIVE"
    | "ACCESSIBILITY"
    | "FUNCTIONAL"
    | "PERFORMANCE"
    | "BRAND"
  priority: "BLOCKING" | "HIGH" | "NORMAL"
  verification_method:
    | "DOM"
    | "BROWSER_ASSERTION"
    | "SCREENSHOT_REVIEW"
    | "ACCESSIBILITY"
    | "PERFORMANCE"
    | "FUNCTIONAL_TEST"
    | "MODEL_CRITIQUE"
    | "HUMAN_REVIEW"
}
```

Example:

```text
Criterion:
Mission status must remain readable at a glance while the terminal is closed.

Priority:
HIGH

Verification:
Screenshot review + DOM information-density check
```

This is stronger than:

```text
make it premium.
```

---

## H22. Canonical `DesignGrammar`

```ts
type DesignGrammar = {
  grammar_id: string
  project_id: string
  version: number

  typography: {
    families: string[]
    scale_ref?: string
    weight_roles: Record<string, string>
    line_height_rules: string[]
    max_line_length_rules: string[]
  }

  spacing: {
    base_unit?: number
    scale: number[]
    density_rules: string[]
  }

  surfaces: {
    hierarchy: string[]
    border_rules: string[]
    radius_scale: string[]
    elevation_rules: string[]
  }

  color: {
    semantic_roles: Record<string, string>
    theme_behavior: string[]
    prohibited_patterns: string[]
  }

  layout: {
    navigation_model: string
    container_rules: string[]
    grid_rules: string[]
    breakpoint_rules: string[]
  }

  components: {
    preferred_patterns: string[]
    avoided_patterns: string[]
    reuse_rules: string[]
  }

  interaction: {
    feedback_rules: string[]
    focus_rules: string[]
    motion_rules: string[]
  }

  content: {
    tone_rules: string[]
    prohibited_copy_patterns: string[]
  }

  assets: {
    iconography_rules: string[]
    imagery_rules: string[]
  }

  provenance_refs: string[]
  freshness_dependencies: string[]
}
```

The grammar should reflect existing project reality when one exists. It is not a generic design-system generator that overwrites a mature product identity.

---

## H23. Design Decision Provenance

A design fact should be traceable to one of:

```text
EXPLICIT_USER
EXISTING_CODE
EXISTING_RENDER
BRAND_ASSET
REFERENCE_ANALYSIS
ACCESSIBILITY_REQUIREMENT
FUNCTIONAL_REQUIREMENT
DESIGN_DIRECTOR_INFERENCE
```

For example:

```text
"Use small radii"
```

should record whether it came from:

- an explicit user instruction;
- existing design tokens;
- repeated component evidence;
- a new inferred design direction.

Inferred design decisions have lower authority than explicit user constraints and mature existing product conventions.

---

## H24. Existing Design Detection Pipeline

Before a substantial redesign, AgentCode should perform:

```text
1. detect frontend framework/workspaces
2. locate global styles/theme files
3. detect component libraries
4. detect design tokens/CSS variables
5. identify fonts and asset pipeline
6. map layout/navigation components
7. map high-reuse primitives
8. start app if feasible
9. capture representative baseline screens
10. extract repeated visual rules
11. compare code-derived rules against rendered reality
12. build/update DesignGrammar
```

### Framework detection record

```ts
type FrontendProfile = {
  frameworks: string[]
  rendering_model?: string
  styling_systems: string[]
  component_libraries: string[]
  router?: string
  build_system?: string
  design_token_sources: string[]
  font_sources: string[]
  asset_roots: string[]
  source_mapping_capability:
    | "HIGH"
    | "PARTIAL"
    | "LOW"
    | "UNAVAILABLE"
}
```

Framework detection is evidence-based and may be `PARTIAL`; the system must not assume every TypeScript repository is React/Next.js.

---

## H25. Human Design Overrides and Protected Areas

The user must be able to define:

```text
Preserve this navigation.
Do not change logo placement.
Do not touch checkout flow.
Keep existing typography.
Redesign everything except chart components.
```

Represent these as durable `DesignConstraint` records.

```ts
type DesignConstraint = {
  constraint_id: string
  scope: string[]
  instruction: string
  authority: "EXPLICIT_USER" | "PROJECT_POLICY" | "INFERRED"
  enforcement: "HARD" | "SOFT"
  created_at: string
}
```

A hard constraint may not be silently violated because a visual critic prefers another design.

If implementation discovers that a hard constraint makes another blocking requirement impossible, the Design mission creates a human escalation rather than ignoring the constraint.

---

## H26. Design Artifact Layout

Design artifacts should use a predictable project-local AgentCode structure where policy allows.

Recommended logical structure:

```text
.agentcode/
└── design/
    ├── DESIGN_STATE.md
    ├── briefs/
    │   └── <brief-id>.json
    ├── grammar/
    │   └── <grammar-version>.json
    ├── references/
    │   └── manifests/
    ├── iterations/
    │   └── <mission-id>/
    │       ├── iteration-001.json
    │       ├── iteration-002.json
    │       └── ...
    └── evidence/
        └── <mission-id>/
```

Raw screenshots may instead live in AgentCode-managed artifact storage to avoid polluting the repository. The exact physical storage location is a constrained decision, but identifiers and retention semantics must remain stable.

---

## H27. `DESIGN_STATE.md` Generation Contract

`DESIGN_STATE.md` must be generated from structured state.

Required header:

```markdown
# Project Design State

Generated At:
Repository View:
Commit:
Design Grammar Version:
Design Brief Version:
Freshness:
```

Required content:

```text
Product personality
Target users/workflows
Current design principles
Tokens
Layout/navigation rules
Component conventions
Accessibility constraints
Responsive rules
Protected/do-not-touch areas
Avoided patterns
Accepted design decisions
Known design debt
Current representative screens
```

It is a handoff snapshot, not the source of truth.

---

## H28. Reference Analysis Record

A reference image/site/moodboard becomes:

```ts
type ReferenceAnalysis = {
  reference_id: string
  source_type: "SCREENSHOT" | "URL" | "IMAGE" | "MOODBOARD" | "SKETCH"
  artifact_ref: string
  license_or_origin_note?: string
  extracted: {
    hierarchy: string[]
    layout: string[]
    spacing: string[]
    typography: string[]
    navigation: string[]
    component_behavior: string[]
    motion: string[]
    visual_motifs: string[]
  }
  explicitly_do_not_copy: string[]
  adopted_principles: string[]
}
```

### Copying boundary

AgentCode may learn:

```text
dense top navigation
editorial typography
split-pane information hierarchy
compact status treatment
```

but should not reproduce proprietary logos, illustrations, copied marketing text, protected assets, or a product's complete distinctive trade dress without user-supplied rights.

---

## H29. Asset and Licensing Policy

Before adding a new design asset dependency, AgentCode should classify it:

```text
PROJECT_OWNED
USER_SUPPLIED
PERMISSIVE
LICENSE_REVIEWED
UNKNOWN
```

Unknown third-party images/icons/fonts should not be silently copied into production.

### Fonts

Prefer in order:

1. existing project font;
2. system stack when appropriate;
3. properly licensed/open web font;
4. user-provided licensed asset.

### Icons

Prefer existing project icon set before introducing another package.

### Image generation

Generated imagery should be recorded as generated content and not falsely attributed to an existing brand.

---

## H30. Preview Environment Isolation

Design preview must not casually reuse the user's personal browser profile.

Each preview session should have:

```text
isolated browser context
isolated cookies/storage
explicit environment variables
known dev-server origin
synthetic/test data by default
network policy
artifact sensitivity classification
```

Production credentials should not be injected into preview merely to make the screen look populated.

When real backend integration is required, use a deliberate environment profile and Doc 04 Secret Broker.

---

## H31. Preview Session Contract

```ts
type DesignPreviewSession = {
  preview_session_id: string
  mission_id: string
  repository_view: string
  commit_or_worktree_ref: string
  command_ref: string
  process_ref: string
  origin: string
  port: number
  browser_session_ref: string
  environment_profile: string
  started_at: string
  last_activity_at: string
  status:
    | "STARTING"
    | "READY"
    | "DEGRADED"
    | "STOPPING"
    | "STOPPED"
    | "FAILED"
}
```

### Port lifecycle

Ports must be allocated through Doc 04's process/port management. A crashed preview must release or reconcile its reservation. The UI must not assume port 3000.

### Readiness

Preview becomes `READY` only after:

```text
process alive
+
HTTP readiness
+
target route loadable
```

A process merely listening does not guarantee the app is usable.

---

## H32. Browser Resource Governor for Design

On the target 8 GB Mac, Design Studio must avoid keeping many Chromium contexts, local vision models, LSPs, indexers, and Workers active simultaneously.

Initial policy:

```text
1 interactive preview browser session per active Design mission
1 visual-QA inference at a time by default
reuse page/context across iterations when safe
close superseded preview contexts
stop idle preview after configurable timeout
```

Under memory pressure:

```text
pause optional visual analysis
close stale tabs/pages
unload local visual model
reduce screenshot history in memory
retain artifacts on disk
```

The daemon decides resource admission; the UI simply reflects:

```text
Visual analysis queued due to memory pressure
```

if necessary.

---

## H33. Design Iteration Record

Every meaningful iteration records:

```ts
type DesignIteration = {
  iteration_id: string
  mission_id: string
  ordinal: number
  brief_version: number
  grammar_version: number
  base_repository_ref: string
  change_set_ref?: string

  screenshot_refs: string[]
  browser_check_refs: string[]
  visual_critique_ref?: string
  accessibility_ref?: string
  performance_ref?: string
  functional_verification_ref?: string

  issues_found: string[]
  issues_resolved: string[]
  accepted_deviations: string[]

  created_at: string
}
```

Minor CSS tweak loops need not create a permanent screenshot each time. Record meaningful checkpoints.

---

## H34. Screenshot and Trace Retention

Screenshots are evidence and can also contain sensitive information.

Each screenshot record should include:

```text
artifact_id
project_id
mission_id
iteration_id
repository view
commit/worktree
route
viewport
theme
browser version
timestamp
sensitivity
redaction state
```

### Default retention

`BENCHMARK_PENDING` initial behavior:

- preserve baseline;
- preserve final accepted screenshots;
- preserve screenshots tied to unresolved findings;
- retain a bounded number of intermediate iterations;
- garbage-collect redundant intermediates after mission completion.

### Redaction

Before sending screenshots to remote visual models, detect/obscure where feasible:

```text
secret values
tokens
private keys
password fields
sensitive test data
production customer information
```

If safe redaction cannot be guaranteed, routing must obey Doc 01/02 trust and sensitivity policies or use a local model.

---

## H35. Visual Critique Record

A visual critic must output structured findings rather than “looks good.”

```ts
type VisualCritique = {
  critique_id: string
  iteration_id: string
  source:
    | "DETERMINISTIC"
    | "LOCAL_VISION_MODEL"
    | "REMOTE_VISION_MODEL"
    | "HUMAN"
  findings: VisualFinding[]
  quality_summary: string
  unresolved_uncertainty: string[]
}
```

```ts
type VisualFinding = {
  finding_id: string
  category:
    | "LAYOUT"
    | "HIERARCHY"
    | "TYPOGRAPHY"
    | "SPACING"
    | "COLOR"
    | "RESPONSIVE"
    | "ACCESSIBILITY"
    | "CONSISTENCY"
    | "GENERIC_PATTERN"
    | "BRAND_MISMATCH"
    | "COPY"
    | "PERFORMANCE"
    | "FUNCTIONAL_VISUAL"
  severity: "BLOCKING" | "HIGH" | "MEDIUM" | "LOW" | "INFO"
  confidence: "HIGH" | "MEDIUM" | "LOW"
  description: string
  evidence_refs: string[]
  affected_region?: string
  criterion_refs: string[]
  recommended_direction?: string
  status: "OPEN" | "DISMISSED" | "FIXED" | "ACCEPTED"
}
```

---

## H36. Visual Quality Rubric

Design acceptance should use concrete dimensions instead of one arbitrary beauty score.

Recommended rubric dimensions:

| Dimension | Questions |
|---|---|
| Product fit | Does the interface communicate the actual product and workflows? |
| Hierarchy | Are primary actions/states visually dominant for the right reasons? |
| Layout | Is spatial organization coherent and efficient? |
| Typography | Is scale/weight/line-height intentional and readable? |
| Spacing | Is rhythm consistent without excessive emptiness? |
| Interaction | Are controls understandable and state changes clear? |
| Brand/grammar | Does the result respect the project grammar? |
| Responsiveness | Does structure adapt rather than merely shrink? |
| Accessibility | Are keyboard, focus, semantics, contrast and motion acceptable? |
| Information density | Is the density appropriate to the product rather than template-driven? |
| Originality | Does it avoid generic default composition without becoming weird? |
| Functional preservation | Did the redesign retain required behavior? |
| Performance | Did visual implementation avoid unjustified cost? |

A visual model may score dimensions internally, but **blocking acceptance is criterion/finding based**, not “score >= 85.”

---

## H37. Anti-Slop Heuristics as Evidence

The anti-slop system should combine deterministic and model-based signals.

### Deterministic candidates

Examples:

```text
many sibling cards with identical structure
very large hero height relative to content
excessive radius values inconsistent with grammar
multiple decorative gradients
all sections wrapped in independent rounded containers
very low text/data density in professional/admin context
repeated placeholder marketing phrases
```

These signals create critique candidates, not automatic failures.

### Model-based critique

The critic asks:

```text
Could this layout belong to almost any SaaS?
Which parts communicate this specific product?
Which components appear to be untouched library defaults?
Which visual decisions have no functional or brand justification?
```

A finding becomes blocking only when it conflicts with the brief or quality criteria.

---

## H38. Visual Diff and False-Positive Handling

Pixel-level differences are not inherently regressions.

Visual comparison should classify change regions:

```text
EXPECTED_CHANGED_REGION
EXPECTED_LAYOUT_REFLOW
UNEXPECTED_CHANGED_REGION
BASELINE_ENVIRONMENT_DRIFT
FONT_RENDERING_VARIANCE
ANIMATION_TRANSIENT
UNKNOWN
```

### Comparison inputs

```text
same route
same viewport
same theme
same seeded data
same or compatible browser rendering environment
```

### Widening rule

If a change touches global tokens/layout primitives, compare representative pages outside the target screen because unintended regressions may appear elsewhere.

### Human baseline approval

Repositories using strict screenshot snapshots may preserve project-native baseline workflow. AgentCode must not rewrite snapshots solely to make tests green without explaining the intended visual change.

---

## H39. Responsive Test Matrix

The old `mobile/tablet/desktop` guidance becomes a task-derived matrix.

```ts
type ViewportProfile = {
  name: string
  width: number
  height: number
  device_scale_factor?: number
  touch?: boolean
  required: boolean
  reason: string
}
```

For a responsive web product, initial representative profiles may include:

```text
small phone
large phone
tablet/compact
desktop
large desktop
```

Exact widths are `CONSTRAINED_IMPLEMENTATION_DECISION` or project-derived.

### Required responsive assertions

Where applicable:

- no unintended horizontal overflow;
- primary action remains reachable;
- nav remains operable;
- dialogs fit viewport;
- text does not become unusably narrow;
- tables have deliberate overflow/alternate layout;
- focus/keyboard behavior remains valid;
- touch target size remains usable;
- sticky/fixed elements do not occlude content.

AgentCode desktop itself should test relevant desktop window sizes instead of pretending it is a mobile app.

---

## H40. Accessibility Target

For AgentCode's own V1 interface, the intended target is:

> **WCAG 2.2 AA for primary workflows where technically applicable.**

This is a quality target, not a claim that automated tools can prove full conformance.

### Automated checks

Use, where applicable:

```text
axe-core/equivalent
accessibility tree
semantic role checks
label relationships
contrast tooling
focus-order assertions
keyboard flows
reduced-motion behavior
```

### Manual/model-assisted review

Some issues require judgement:

```text
meaningful accessible names
logical heading hierarchy
focus placement after modal/navigation actions
error messaging clarity
screen-reader workflow coherence
```

Automated “0 violations” does not equal complete accessibility proof.

---

## H41. AgentCode Desktop Accessibility Contract

Primary workflows must be possible without mouse:

```text
open/switch project
focus goal composer
submit goal
navigate mission summary
open Changes
move through diff files
open Needs You request
respond to request
pause/resume mission
open command palette
```

### Focus rules

- opening dialogs moves focus intentionally;
- closing dialogs restores logical prior focus;
- route changes place focus in meaningful page heading/content;
- progress updates do not steal focus;
- background events do not interrupt screen reader flow unless user-actionable.

---

## H42. Interaction-State Evidence

Design verification should not capture only a single static “happy-path” screenshot.

For changed interactive components, evidence should include relevant states such as:

```text
default
hover
focus-visible
pressed/active
selected
disabled
loading
empty
error
success
permission-denied
offline/degraded
```

Only applicable states are required.

A redesigned form with a beautiful default screen but unreadable validation errors is incomplete.

---

## H43. Functional Preservation Matrix

Before a redesign, identify functions that must survive.

Example:

| Surface | Required behavior |
|---|---|
| Login form | submit, validation, error, loading |
| Dashboard nav | route transitions, active state |
| Data table | sort/filter/pagination |
| Modal | open, close, focus trap |
| Settings form | load/save/error |
| Authenticated page | unauthorized redirect |

After redesign, these flows are rerun through Doc 05 verification.

This prevents Design Studio from optimizing screenshots at the expense of software behavior.

---

## H44. Performance Budget for Generated UI

Design Studio should avoid introducing material regressions without justification.

Track project-appropriate signals such as:

```text
bundle delta
new dependency count
large asset bytes
image dimensions/encoding
layout shift
interaction responsiveness
page-load timing
browser memory
animation CPU cost
```

No universal web-performance threshold is locked because projects differ. However, Design Studio must explain large new costs.

Examples requiring justification:

```text
+500 KB client dependency for one animation
5 MB hero image
continuous background WebGL effect
new icon library duplicating existing icons
```

---

## H45. AgentCode Desktop Resource Budget

Because AgentCode targets an 8 GB Mac, the desktop UI itself must remain lightweight.

Initial `BENCHMARK_PENDING` guardrails:

```text
UI renderer should remain comfortably below 1 GB RSS during normal mission monitoring.
Core desktop + daemon excluding optional browser/local-model workloads should target a materially lower steady-state footprint than heavyweight Electron IDEs.
Mission monitoring must remain responsive under system memory pressure.
```

The exact final release thresholds belong in Doc 10 after measurement.

### UI responsiveness goals

Initial targets:

```text
user input should not visibly block during background indexing
route/panel interactions should remain interactive during mission events
large event streams should use virtualization/pagination
large diffs should use lazy rendering
terminal output should use bounded buffers
```

---

## H46. Diff Viewer Contract

The Changes view should expose:

```ts
type ChangeGroup = {
  group_id: string
  task_id?: string
  change_set_id?: string
  commit_refs: string[]
  title: string
  verification_state: string
  files: DiffFileSummary[]
}
```

Per file show:

```text
path
status
additions/deletions
task/change-set provenance
verification freshness
```

### Large diff behavior

Use:

```text
file virtualization
collapsed unchanged regions
syntax-aware rendering where available
binary/generated-file summaries
lazy loading
```

Do not freeze renderer by loading an enormous monorepo diff into one Monaco instance.

---

## H47. Terminal UX Contract

The terminal is an inspection/control surface over Doc 04 processes.

Each terminal tab maps to a known process/session.

The UI should clearly distinguish:

```text
AgentCode-managed process
user-created terminal
read-only historical output
```

Closing a terminal viewport must not automatically kill a long-running AgentCode-managed dev server.

Conversely, killing a process through terminal controls must go through Process Manager rather than sending an untracked renderer-side signal.

---

## H48. Human Escalation UX

A `Needs You` card should answer:

```text
What is blocked?
Why can't AgentCode resolve it autonomously?
What options exist?
Which option is recommended?
What happens if I ignore this?
```

Example:

```text
Production migration requires approval

AgentCode has prepared and verified the migration locally.
Applying it would modify production data.

Recommended: review the migration, then approve deployment.

[Review Migration]
[Approve]
[Keep Paused]
```

### Deduplication

The UI should not present repeated prompts for the same durable human request.

If several tasks are blocked by one missing credential, show one request and affected work summary.

---

## H49. Pause, Resume, Cancel and Quit UX

### Pause

User action:

```text
Pause
```

UI:

```text
Pausing safely…
```

then authoritative:

```text
Paused
```

The renderer must not claim pause before Kernel confirms safe state.

### Resume

One action. Reconciliation occurs in daemon; UI may show:

```text
Checking repository and recovering state…
```

### Cancel

Before cancellation, show consequence:

```text
Stop this mission?
Completed local changes will be preserved.
Pending work will stop safely.
```

Default cancellation does not destructively roll back Git.

### Window close

Window close ≠ daemon quit.

### Full quit

If active missions exist:

```text
Quit AgentCode background service?
Active missions will stop after reaching a safe checkpoint.

[Keep Working in Background]
[Quit Service]
```

---

## H50. Undo and Revert UX

There are two distinct concepts.

### Design iteration revert

Revert a specific design ChangeSet/checkpoint through Doc 04 Git/Edit mechanisms.

### Manual UI undo

Small text/editor changes made directly by user may use local editor undo.

Never implement a giant opaque “undo AgentCode” that rewrites repository state without showing which commits/change sets will change.

Design history should permit:

```text
Compare
Revert to iteration
Create new branch from iteration
```

where underlying Git state supports it.

---

## H51. Notifications as a Kernel-Derived Projection

Notification types:

```text
MISSION_COMPLETE
HUMAN_ACTION_REQUIRED
MISSION_BLOCKED_OR_FAILED
```

### Suppressed by default

```text
provider switched
worker/session restarted
transient test failed
context compacted
scanner finished
task started
```

unless the event creates user action.

### Completion invariant

Notification may fire only after committed Kernel `COMPLETE`.

Renderer state such as:

```text
all visible tasks appear green
```

is insufficient.

---

## H52. Notification Privacy

Desktop notifications may appear on lock screens.

Therefore notification body should avoid sensitive:

```text
source code
secret values
internal exploit details
customer data
private URLs
```

Default completion notification:

```text
AgentCode
Mission complete for <project>
```

Detailed counts may be shown if project privacy policy permits.

Security notifications should say:

```text
Security review needs your attention
```

rather than exposing a critical vulnerability description on the OS lock screen.

---

## H53. Sound Contract

Sound preferences are local UI settings.

```ts
type SoundSettings = {
  completion_enabled: boolean
  attention_enabled: boolean
  blocked_enabled: boolean
}
```

V1 should use short packaged application sounds or native notification sounds.

Do not:

```text
download sounds dynamically
bundle copyrighted audio without rights
play a sound for every task
```

Respect system audio state.

---

## H54. Settings Schema

The settings UI should map to a versioned configuration schema rather than ad hoc toggles.

Logical top-level configuration:

```ts
type UserSettings = {
  schema_version: number

  appearance: {
    theme: "SYSTEM" | "LIGHT" | "DARK"
    accent?: string
    density?: "COMFORTABLE" | "COMPACT"
  }

  notifications: {
    completion: boolean
    needs_user: boolean
    blocked: boolean
    sounds: SoundSettings
  }

  mission_defaults: {
    autonomy_profile: "STANDARD" | "HIGH_AUTONOMY" | "RESTRICTED"
    paid_budget_policy_ref?: string
    security_depth?: string
    parallelism_override?: number
  }

  privacy: {
    show_sensitive_notification_details: boolean
    screenshot_retention?: string
  }

  developer: {
    show_advanced_details_by_default: boolean
    terminal_font_size?: number
  }
}
```

Provider secrets themselves are never stored in the UI preferences file.

---

## H55. Autonomy Profile UX

Friendly profiles are UI aliases over policy.

The UI should show a short explanation.

### Standard

```text
Routine local development runs automatically.
External or high-impact actions ask when necessary.
```

### High Autonomy

```text
Trusted project workflows can perform more pre-approved operations automatically.
Production safety boundaries remain active.
```

### Restricted

```text
Read-heavy mode with limited writes, network access, and more approvals.
Recommended for unknown repositories.
```

The UI should not imply that High Autonomy disables security policy.

---

## H56. Project Settings and Instruction Visibility

Advanced project settings may show:

```text
Project Instructions
Trusted Skills
Permissions
Provider Policy
Security Policy
Design Constraints
```

For nested instructions, expose provenance:

```text
AGENTS.md
applies to: repository

packages/api/AGENTS.md
applies to: packages/api/**
```

Do not flatten contradictory scoped rules into one misleading text box.

---

## H57. Empty, Loading, Error and Degraded States

Every first-class surface needs deliberate state handling.

### Projects

```text
empty: no repositories yet
loading: recent project metadata loading
error: Library/daemon unavailable
```

### Mission

```text
planning
working
paused
blocked
needs user
complete
cancelled
recovery
```

### Discuss

```text
repo indexing incomplete
model unavailable
read-only context degraded
```

### Design

```text
preview starting
preview crashed
visual model unavailable
browser unavailable
functional verification failed
```

### Security

```text
scanner missing
scan partial
authorization expired
scope blocked
finding store unavailable
```

A degraded optional subsystem must not produce a generic full-product error screen.

---

## H58. Error Message Schema

User-facing errors should be derived from structured errors:

```ts
type UserFacingError = {
  code: string
  title: string
  impact: string
  recovery_state:
    | "RECOVERED"
    | "RETRYING"
    | "BLOCKED"
    | "NEEDS_USER"
    | "FATAL"
  action?: {
    label: string
    command: string
  }
  diagnostics_ref?: string
}
```

Example:

```text
Browser preview stopped unexpectedly.

AgentCode preserved your design changes and will restart the preview.

No action required.
```

rather than:

```text
playwright browser process exit code 1
```

Raw detail remains in diagnostics.

---

## H59. Product Copy Safety and Claim Integrity

AgentCode Design Studio should not fabricate product claims merely to fill marketing UI.

When generating website copy, classify content:

```text
USER_PROVIDED_FACT
REPOSITORY_SUPPORTED_FACT
PLACEHOLDER
CREATIVE_TAGLINE
UNVERIFIED_PRODUCT_CLAIM
```

Do not invent:

```text
"Trusted by 50,000 teams"
"99.99% uptime"
"Military-grade security"
"#1 platform"
```

without supporting evidence.

Preview placeholders must be visibly marked in development artifacts and not silently ship as factual production content.

---

## H60. Real Data, Synthetic Data and Privacy

Design preview data source must be labeled:

```text
REAL_LOCAL_TEST_DATA
SYNTHETIC_DATA
MOCK_DATA
PRODUCTION_DATA
```

Production data should be avoided for visual design iteration.

If production-like datasets are needed, use sanitized/synthetic fixtures when possible.

The UI must not render sensitive data into screenshots sent to remote visual models without trust-policy approval.

---

## H61. Command Palette Contract

The command palette is an expert shortcut, not a second architecture.

Commands should be registered with:

```ts
type UiCommand = {
  command_id: string
  title: string
  keywords: string[]
  scope: "GLOBAL" | "PROJECT" | "MISSION" | "DESIGN" | "SECURITY"
  enabled_when: string[]
  action: string
}
```

Initial high-value commands:

```text
Open Project
Switch Project
Start Mission
Open Current Mission
Discuss Repository
Start Design Mission
Start Security Audit
Pause Mission
Resume Mission
Review Changes
Open Diagnostics
```

Commands unavailable in the current state should explain why.

---

## H62. Keyboard Shortcut Rules

Use platform-native conventions.

Required goals:

- no shortcut should override standard text editing unexpectedly;
- every shortcut must have menu/command-palette equivalent;
- shortcuts must be discoverable;
- reserved OS shortcuts must be avoided;
- accessibility users can complete workflows without memorizing shortcuts.

Initial mappings may remain as in the base document, subject to platform conflict testing.

---

## H63. Large Mission History UX

Long missions may contain thousands of events.

The UI should support:

```text
virtualized timeline
category filters
task filter
search
jump to human requests
jump to failures
jump to verification
jump to final audit
```

Do not deserialize/render the entire raw event log into the DOM at once.

Search results should link to authoritative event/task/finding IDs.

---

## H64. Diagnostics Surface

Diagnostics is advanced and must avoid secret leakage.

Include:

```text
daemon version/health
protocol version
database health summary
project index state
browser/process health
provider route health summary
tool availability
recent error codes
log bundle generation
```

A “Copy Diagnostics” action must redact:

```text
API keys
authorization headers
cookies
secret values
private file content unless explicitly requested
```

---

## H65. Product Timeline Semantics

Project timeline entries should be durable project milestones:

```text
mission completed
mission cancelled
major accepted decision
security audit completed
risk accepted
design mission accepted
release/tag milestone
```

It should not be a copy of every Kernel event.

Timeline is a human navigation layer over authoritative records.

---

## H66. Design Source Mapping Capability

Direct visual selection is `OPTIONAL_V1`.

Capability levels:

```text
UNAVAILABLE
DOM_ONLY
FILE_HINT
COMPONENT_RESOLVED
SYMBOL_RESOLVED
```

The UI must show uncertainty.

Example:

```text
Likely source:
src/components/Header.tsx
```

is better than falsely asserting exact mapping when source maps/framework transforms make certainty low.

Only supported frameworks with verified mapping should expose one-click source targeting as a confident feature.

---

## H67. Direct Visual Selection Request

When available:

```ts
type VisualSelection = {
  selection_id: string
  preview_session_id: string
  route: string
  dom_locator: string
  bounding_box: { x: number; y: number; width: number; height: number }
  text_excerpt?: string
  component_name?: string
  source_file?: string
  source_range?: string
  confidence: "HIGH" | "MEDIUM" | "LOW"
}
```

The resulting edit still goes through normal Context Engine, Edit Engine, Git and Verification. Clicking an element does not grant the renderer direct source-write authority.

---

## H68. Design Acceptance State Machine

A Design iteration has:

```text
DRAFT
→ IMPLEMENTED
→ PREVIEW_READY
→ VISUAL_REVIEW
→ FUNCTIONAL_REVIEW
→ REPAIR
→ VERIFIED
→ ACCEPTED
```

Allowed outcomes also include:

```text
REJECTED
SUPERSEDED
BLOCKED
```

`VERIFIED` means automated/independent checks satisfy the current design verification profile.

`ACCEPTED` means the iteration is the mission's selected design result. For fully autonomous missions, Kernel/verification policy may accept after all hard criteria pass. If user selection was explicitly required, acceptance waits for the user.

---

## H69. Design Verification Profile

Every substantial Design mission derives a profile from:

```text
target screens
framework
responsive targets
existing test capability
brief criteria
changed shared primitives
accessibility target
performance sensitivity
functional workflows
```

Example:

```text
SCREENSHOT_BASELINE
DOM_OVERFLOW_CHECK
DESKTOP_1280
DESKTOP_1600
KEYBOARD_FLOW
AXE_CORE
LOGIN_FLOW
NAVIGATION_FLOW
GLOBAL_TOKEN_REGRESSION_PAGES
```

The UI may summarize:

```text
Visual, responsive, accessibility and functional verification enabled
```

without showing the whole manifest by default.

---

## H70. Design Critic Independence

Where routing budget permits:

```text
Implementer model family != Visual Critic model family
```

is preferred for substantial design work.

However, deterministic browser/accessibility evidence remains important even when only one visual model is available.

If visual independence degrades:

```text
visual_independence = DEGRADED
```

should be recorded in evidence rather than falsely claiming independent review.

---

## H71. AgentCode Self-Design Standard

The AgentCode desktop should dogfood Design Studio principles.

Its own UI should be evaluated for:

```text
minimal surface
progress truthfulness
status comprehension
keyboard accessibility
responsive desktop resizing
high-density diff/activity surfaces
dark/light/system appearance
error/recovery clarity
no generic SaaS decoration
performance under background work
```

The product should not enforce anti-slop standards on generated applications while shipping an obviously generic or noisy desktop shell.

---

## H72. AgentCode Desktop Visual Grammar

The V1 AgentCode application itself should begin from these product-specific principles:

```text
calm technical surfaces
compact but readable density
restrained radius
minimal decorative gradients
strong typography hierarchy
subtle borders/surface separation
one restrained accent
semantic status colors
code/diff content gets visual priority when opened
large empty marketing-style cards avoided
animations short and functional
```

This is not a final pixel spec. It is the baseline grammar against which the product shell should be designed.

---

## H73. Desktop Layout Baseline

Recommended desktop shell:

```text
┌─────────────────────────────────────────────────────────┐
│ App / Project Context                         Status     │
├───────────────┬─────────────────────────────────────────┤
│ Primary Nav   │ Main Surface                            │
│               │                                         │
│ Projects      │                                         │
│ Goal          │                                         │
│ Discuss       │                                         │
│ Design        │                                         │
│ Security      │                                         │
│               │                                         │
├───────────────┴─────────────────────────────────────────┤
│ Optional compact status / background mission indicator │
└─────────────────────────────────────────────────────────┘
```

The exact nav orientation may change if UX testing proves another structure materially better, but V1 should avoid permanently visible multi-pane IDE complexity.

Mission `Changes`, `Activity`, and `Details` belong within the mission surface rather than permanently consuming global navigation space.

---

## H74. Window Size and Responsive Desktop Behavior

AgentCode is a desktop application, but it must handle resizing.

Define:

```text
minimum usable width
compact desktop width
normal desktop width
wide desktop width
```

At compact width:

- labels may collapse to icons where accessible;
- secondary metadata hides before primary workflow;
- Changes/detail panels become overlays or stacked views;
- mission status remains visible;
- no horizontal app-shell overflow.

Exact pixel thresholds are benchmark/implementation decisions.

---

## H75. Theme Contract

Theme values:

```text
SYSTEM
LIGHT
DARK
```

System is default.

Theme changes should not require daemon restart.

The Design Studio preview theme is separate from AgentCode shell theme: a user may run AgentCode dark while testing a project's light theme.

### Contrast

Semantic colors must remain distinguishable in both themes. Do not encode status by color alone.

---

## H76. UI Component Strategy

AgentCode may use a component library, but the library must not define product identity.

Policy:

```text
use primitives for accessibility/behavior
customize visual grammar
avoid untouched default library look
avoid creating a giant internal design system before need
```

Shared AgentCode primitives likely include:

```text
Button
IconButton
Input
Textarea
Tabs
Popover
Dialog
Menu
Tooltip
Badge/Status
List
VirtualList
SplitView
DiffSurface
CodeSurface
EmptyState
ErrorState
NotificationBanner
```

Component-library selection is constrained implementation detail and should be validated in Doc 07.

---

## H77. Visual Asset Policy for AgentCode Itself

AgentCode's own brand assets should be intentionally created/owned.

Do not reuse:

```text
donor repository logos
third-party UI screenshots
proprietary icon artwork
```

as product branding.

Reference products inform interaction patterns, not AgentCode identity.

---

## H78. Design Studio Failure Taxonomy

Design-specific failures should be distinguishable:

| Failure | Typical recovery |
|---|---|
| `PREVIEW_COMMAND_MISSING` | detect alternate project-native command or Needs You |
| `PREVIEW_BUILD_FAILED` | repair code/config |
| `PREVIEW_PORT_CONFLICT` | allocate new port |
| `BROWSER_CRASHED` | restart browser session |
| `ROUTE_NOT_FOUND` | inspect router/project structure |
| `SCREENSHOT_FAILED` | retry browser evidence, not whole mission |
| `VISUAL_MODEL_UNAVAILABLE` | deterministic checks/local/alternate model |
| `ACCESSIBILITY_TOOL_MISSING` | degrade with explicit evidence gap |
| `FUNCTIONAL_REGRESSION` | repair task |
| `DESIGN_CONSTRAINT_CONFLICT` | replan or human escalation |
| `REFERENCE_UNREADABLE` | ask only if reference is essential |
| `ASSET_LICENSE_UNKNOWN` | substitute safe asset or block addition |
| `SOURCE_MAPPING_UNAVAILABLE` | fall back to repository search |

The UI should translate these into meaningful recovery messages.

---

## H79. Design Studio Recovery

A browser/visual subsystem crash does not mean design work is lost.

Recovery sequence:

```text
load current DesignBrief/Grammar
load latest ChangeSet/checkpoint
reconcile preview process
restart dev server if needed
restart browser
recreate target route
capture current render
invalidate stale visual evidence
continue critique/verification
```

Previous screenshots tied to older commits remain historical evidence but cannot prove the recovered state.

---

## H80. Security/Design Mode Resource Separation

A project should not accidentally run:

```text
heavy ZAP scan
+
Nuclei
+
browser preview
+
local vision model
+
two coding Workers
```

simultaneously on an 8 GB Mac unless Resource Governor explicitly admits it.

The UI may show:

```text
Security scan queued while Design preview is active
```

rather than launching everything and degrading the machine.

---

## H81. Privacy Boundaries in UX

The UI should visually distinguish when an action may send project content externally.

Most users should not be interrupted for every normal trusted-provider call, but settings/details should make routing policy inspectable.

For especially sensitive operations:

```text
security evidence
screenshots containing private data
proprietary design references
```

provider-trust policy may require a trusted/local route.

If no compliant route exists, `Needs You` explains the privacy blocker.

---

## H82. Marketing/Design References from URLs

When the user supplies a public website URL as a design reference, AgentCode may inspect it through browser/web tools subject to policy.

It should record:

```text
reference URL
capture time
screens used
principles extracted
```

and should not assume the site remains unchanged forever.

External website content is untrusted input and cannot override project instructions.

---

## H83. Mission/Design Draft Persistence

Draft user text may be locally autosaved so an accidental renderer crash does not lose a long mission or design prompt.

Autosaved drafts should be:

```text
local
project-scoped
not sent to model before submit
replaceable/deletable
```

Sensitive drafts follow local data privacy policy.

---

## H84. Multi-Mission Product Behavior

V1 may display multiple projects and historical missions, but the UI should not imply unlimited simultaneous autonomous execution.

If the Kernel/resource policy admits only limited active missions/workers, the project list can show:

```text
Working
Queued
Paused
Needs You
Complete
```

The UI reflects scheduler truth.

---

## H85. Search and Navigation Identity

Every navigable mission object should have a stable identifier:

```text
mission
task
requirement
finding
decision
event
file
change set
```

Search results should resolve to the authoritative detail surface.

Avoid search indexes based only on display labels because duplicate task titles are possible.

---

## H86. Accessibility of Visualizations

Attack graphs, progress visualizations, and design comparison views require non-visual equivalents.

Example attack path:

```text
Public API
→ Missing Ownership Check
→ Object Key Exposure
→ Private Storage Access
```

must be available as accessible ordered text, not only graph nodes.

Progress bars require textual labels.

Diff status must not be encoded only by green/red colors.

---

## H87. Design Comparison UX

Multiple directions are `OPTIONAL_V1`.

If implemented, compare at brief/wireframe or bounded prototype level.

Comparison card should explain:

```text
direction intent
strengths
trade-offs
implementation cost
accessibility risk
fit with current product
```

The default autonomous path may choose the strongest direction without asking the user when the mission explicitly allows autonomous design decisions.

---

## H88. Product Telemetry and Privacy

Core AgentCode V1 does not require cloud analytics to function.

If product telemetry is later added:

```text
opt-in/transparent policy
no source code
no prompts by default
no secret values
no vulnerability payloads
```

Local operational metrics used by Kernel/verification are not the same thing as vendor analytics.

---

## H89. UI Logging

Renderer logs should include:

```text
route transitions
IPC failures
render errors
event revision gaps
preview UI failures
```

but not:

```text
full prompts
full model responses
secret values
authorization headers
raw source files
```

unless explicit debug capture is enabled with redaction.

---

## H90. V1 Capability Scope Matrix

### `REQUIRED_V1`

- Projects/recent projects.
- Project bootstrap/readiness display.
- Goal Mode and mission composer.
- Mission summary/status with truthful Kernel-derived progress.
- Needs You, pause, resume, cancel.
- Changes and high-quality diff review.
- Activity with progressive disclosure.
- Advanced detail access to context/routing/worktrees/evidence.
- Discuss Mode, read-only by default.
- Discuss → durable decision/plan/mission.
- Design Mode with product understanding, brief, grammar, implementation, preview, critique, responsive/accessibility/functional verification.
- Security Mode entry and clear scope presentation.
- Findings presentation with evidence/attack path.
- Background daemon/window-close behavior.
- Completion/Needs You notifications and subtle sounds.
- Dark/light/system appearance.
- Keyboard-accessible core workflows.
- UI crash isolation and snapshot-then-stream reconnect.
- AgentCode self-UX quality verification.
- Design state persistence/freshness.
- Preview isolation and resource-conscious browser lifecycle.

### `REQUIRED_IF_APPLICABLE`

- Mobile/tablet viewport testing for responsive applications.
- Dark/light validation when target project supports both.
- Cloud/security active-scope UI when such testing is requested.
- Existing screenshot baseline compatibility when repository already uses it.
- Performance checks appropriate to changed UI.
- AI/privacy-sensitive screenshot routing.

### `OPTIONAL_V1`

- Direct DOM-to-source visual selection.
- Multiple design direction comparison.
- Advanced pixel/visual baseline management.
- Rich attack-path graph visualization beyond accessible list.
- Advanced project timeline visualizations.
- Optional live model output.

### `POST_V1`

- Full IDE replacement.
- Shared real-time team mission dashboard.
- Remote multi-user daemon control.
- Plugin marketplace UX.
- Cross-device project synchronization.
- Universal framework-perfect DOM-to-source mapping.
- Continuous product analytics platform.

---

## H91. Expanded Product UX Acceptance Catalog

The high-level tests in Sections 166–188 remain useful. The following stable IDs make the UX implementation executable.

| ID | Scenario | Required result |
|---|---|---|
| `UX-BOOT-001` | first launch with no projects | user can open repository without tutorial maze |
| `UX-BOOT-002` | optional provider missing but local route exists | project opens; setup does not block |
| `UX-BOOT-003` | no model route exists | one actionable setup blocker, not provider-dashboard dump |
| `UX-REC-001` | renderer killed during mission | daemon continues; UI restores authoritative state |
| `UX-REC-002` | event stream revision gap | UI reloads snapshot instead of applying inconsistent events |
| `UX-REC-003` | daemon temporarily unavailable | UI shows reconnect state; does not claim mission failure |
| `UX-MSN-001` | mission plan unstable | percentage hidden; truthful phase shown |
| `UX-MSN-002` | verified work invalidated | progress may decrease and explanation is visible |
| `UX-MSN-003` | Worker says complete but Kernel not complete | no completion UI/notification |
| `UX-MSN-004` | mission committed COMPLETE | summary and notification use committed structured counts |
| `UX-HUM-001` | one credential blocks 4 tasks | one deduplicated Needs You request with affected-work summary |
| `UX-HUM-002` | routine provider failover | no blocking prompt/notification |
| `UX-PAUSE-001` | pause during edit transaction | UI waits for authoritative safe-paused state |
| `UX-QUIT-001` | close window during active mission | daemon remains active |
| `UX-QUIT-002` | full quit during active mission | user receives keep-background/quit-service choice |
| `UX-DIFF-001` | mission changes hundreds of files | Changes remains responsive via virtualization/lazy load |
| `UX-ACT-001` | thousands of tool events | default Activity groups them meaningfully |
| `UX-ACT-002` | verification failure among grouped events | failure remains individually visible |
| `UX-DISC-001` | repository architecture question | response grounded in repo; no writes available |
| `UX-DISC-002` | user says “fix it” in Discuss | explicit promotion to mission rather than silent mutation |
| `UX-DEC-001` | accepted discussion decision | durable decision records provenance |
| `UX-DSN-001` | redesign existing product | baseline product/design inspection occurs before edits |
| `UX-DSN-002` | explicit “do not touch navigation” | navigation remains unchanged or mission escalates conflict |
| `UX-DSN-003` | visual critic says “looks good” without findings/criteria | critique rejected as insufficiently structured |
| `UX-DSN-004` | generic card-grid/gradient output conflicts with brief | anti-slop critique opens findings and iteration continues |
| `UX-DSN-005` | preview port occupied | new port allocated; mission does not fail |
| `UX-DSN-006` | browser crashes after implementation | preview recreated; code/checkpoint preserved |
| `UX-DSN-007` | global design token changed | visual verification widens to representative dependent screens |
| `UX-DSN-008` | baseline screenshot differs due font-render drift only | classified as environment variance rather than automatic regression |
| `UX-DSN-009` | redesign breaks form validation | functional verification fails and repair is required |
| `UX-DSN-010` | reference includes proprietary logo | principles may be used; logo not silently copied |
| `UX-DSN-011` | unknown third-party font/image | production addition blocked/substituted pending license evidence |
| `UX-DSN-012` | screenshot contains synthetic secret | redaction/trust policy prevents unsafe remote routing |
| `UX-DSN-013` | visual source mapping low confidence | UI shows uncertainty; no false exact source claim |
| `UX-DSN-014` | user reverts design iteration | scoped Git/ChangeSet revert; unrelated work preserved |
| `UX-DSN-015` | no visual model available | deterministic/browser/accessibility checks continue; evidence gap explicit |
| `UX-AX-001` | keyboard-only user | core AgentCode workflow remains operable |
| `UX-AX-002` | modal opens/closes | focus moves/restores logically |
| `UX-AX-003` | low contrast seeded | accessibility verification reports blocking/high finding per profile |
| `UX-AX-004` | prefers-reduced-motion enabled | nonessential motion removed/reduced |
| `UX-RSP-001` | compact AgentCode desktop window | primary controls remain usable; no shell overflow |
| `UX-RSP-002` | mobile target page | responsive matrix catches overflow/clipping |
| `UX-PERF-001` | indexing + mission + event stream active | renderer remains responsive |
| `UX-PERF-002` | large diff opened | UI memory/rendering remains bounded |
| `UX-PERF-003` | browser + visual QA under memory pressure | Resource Governor queues/unloads optional work |
| `UX-SEC-001` | passive audit | no active-target authorization ceremony required |
| `UX-SEC-002` | active validation | exact environment/target/rate/destructive policy visible |
| `UX-SEC-003` | production read-only target | active action unavailable/blocked |
| `UX-SEC-004` | finding shown | severity, confidence, proof, evidence and attack path distinct |
| `UX-PRIV-001` | OS lock-screen notification | no source/secret/vulnerability payload leaked |
| `UX-COPY-001` | generated marketing screen | unsupported numerical/product claims flagged or left as placeholder |
| `UX-THEME-001` | system theme changes | AgentCode follows System setting without daemon restart |
| `UX-STATE-001` | project uses loading/error/empty states | redesigned changed components preserve required states |
| `UX-DIAG-001` | diagnostics copied | secrets and auth data redacted |

---

## H92. Implementation Handoff Requirements

An engineer implementing Doc 06 should be able to answer, for every surface:

```text
What authoritative subsystem owns the data?
What is the local view state?
What command does the UI send?
What event confirms success?
What happens after reconnect?
What is the loading state?
What is the degraded state?
What is the error state?
What can the user do next?
What accessibility behavior is required?
What evidence proves the screen works?
```

For every Design Studio work package, the engineer should additionally answer:

```text
What product/design evidence is input?
What DesignBrief version is active?
What DesignGrammar version is active?
What must be preserved?
Which files/components may change?
What preview environment is used?
What browser checks run?
What visual critique criteria apply?
What responsive matrix applies?
What accessibility checks apply?
What functional flows must survive?
What artifacts are retained?
What causes repair?
What constitutes acceptance?
```

If those questions cannot be answered from implementation state, the feature is not ready to be considered complete.

---

## H93. Hardening Revision Summary

Relative to the base document, Hardening Revision 2 specifically adds:

1. decision-status classes and a V1 scope matrix;
2. canonical desktop information architecture;
3. UI/daemon ownership and protocol contracts;
4. snapshot-then-stream reconnect semantics;
5. authoritative vs local UI state separation;
6. truthful progress computation and anti-thrashing behavior;
7. structured Activity and Mission Summary records;
8. first-run/onboarding behavior;
9. repository bootstrap readiness UX;
10. formal Goal/Discuss mode records and transition behavior;
11. durable decision promotion;
12. detailed Security launch/scope/finding UX contracts;
13. canonical Design task types;
14. complete `DesignBrief` schema;
15. brief-derived acceptance criteria;
16. complete `DesignGrammar` schema;
17. design-decision provenance;
18. frontend/framework/design-system detection pipeline;
19. hard `do_not_touch` design constraints;
20. design artifact conventions;
21. generated `DESIGN_STATE.md` contract;
22. reference-analysis and asset-license contracts;
23. isolated preview session and port lifecycle;
24. browser/visual resource governance for the 8 GB Mac;
25. design-iteration records;
26. screenshot retention/sensitivity/redaction;
27. structured Visual Critique and Visual Finding records;
28. design quality rubric;
29. deterministic + model-based anti-slop evidence;
30. visual-diff false-positive handling;
31. responsive test matrices;
32. WCAG 2.2 AA target for primary AgentCode workflows where applicable;
33. interaction-state evidence;
34. functional-preservation matrices;
35. design and desktop performance budgets;
36. diff/terminal/human-escalation contracts;
37. pause/resume/cancel/quit behavior;
38. scoped design rollback behavior;
39. notification privacy;
40. versioned settings schema;
41. command palette and shortcut rules;
42. large-mission history virtualization;
43. diagnostics/redaction requirements;
44. source-mapping confidence levels;
45. Design acceptance state machine and verification profile;
46. AgentCode self-design/dogfooding standards;
47. design-specific failure/recovery taxonomy;
48. privacy boundaries for screenshots/references;
49. expanded stable acceptance-test IDs;
50. implementation handoff questions that prevent “heading-only” completion.

These additions are intentionally implementation-oriented. They do not replace the product philosophy from Sections 1–165; they make that philosophy executable.

---

# 166. Product UX Acceptance Test

A new user should be able to:

```text
open project

type goal

start mission
```

without understanding:

```text
OmniRoute

worktrees

leases

SCIP

Tree-sitter

provider routing
```

---

# 167. Background Mission Acceptance Test

Start mission.

Close main window.

Expected:

```text
mission continues

daemon remains alive

completion notification arrives
```

---

# 168. Minimal UI Acceptance Test

During normal mission operation user should be able to understand status without opening:

```text
raw terminal

provider dashboard

task DAG

model logs
```

---

# 169. Progressive Disclosure Acceptance Test

Advanced user must nevertheless be able to inspect:

```text
task

worker

model

provider

tool call

context pack

raw logs

diff

evidence
```

when needed.

---

# 170. No-Babysitting Acceptance Test

Run representative mission.

Expected:

No routine approval request for:

```text
read

search

edit

format

test

build

local browser QA
```

---

# 171. Notification Acceptance Test

Normal provider failover:

```text
no notification
```

Mission completion:

```text
notification + subtle completion sound
```

Human action required:

```text
notification + distinct subtle sound
```

---

# 172. Discuss Acceptance Test

Ask repository architecture question.

Expected:

AgentCode:

```text
retrieves actual code

examines architecture

discusses grounded answer
```

without editing repository.

---

# 173. Discuss → Mission Acceptance Test

Have design discussion.

Choose:

```text
Execute Plan
```

Expected:

```text
decisions retained

requirements generated

Kernel mission created

no need to restate entire discussion
```

---

# 174. Design Studio Acceptance Test

Given an existing mediocre page:

```text
request premium redesign
```

Expected:

```text
product context analysed

design brief generated

design grammar respected

implementation completed

preview captured

visual review performed

responsive tests performed

functional flow remains intact
```

---

# 175. Anti-Slop Acceptance Test

Give a model freedom to generate a generic SaaS page.

Expected:

Design Critic identifies excessive patterns such as:

```text
generic gradients

default card grids

empty hero space

unmodified component-library look
```

and requests improvement.

---

# 176. Product-Specific Design Acceptance Test

Give identical feature requirements to two intentionally different products.

Expected:

Design output should differ meaningfully according to:

```text
product identity

audience

density

workflow
```

rather than producing nearly identical templates.

---

# 177. Existing Design Preservation Test

Provide repository with a strong existing visual language.

Add a new screen.

Expected:

new screen follows existing system rather than replacing it with generic AI styling.

---

# 178. Design Reference Acceptance Test

Provide screenshot reference.

Expected:

AgentCode extracts:

```text
hierarchy

spacing

layout principles

visual direction
```

without mechanically reproducing proprietary assets.

---

# 179. Responsive Acceptance Test

Design page.

Expected:

AgentCode verifies representative:

```text
mobile

tablet

desktop
```

states and detects overflow/clipping.

---

# 180. Accessibility Acceptance Test

Seed:

```text
missing form label

poor focus state

low contrast
```

Expected:

Design/verification pipeline reports issues.

---

# 181. Browser Feedback Acceptance Test

Introduce visual defect.

Expected:

```text
browser screenshot

visual QA

issue identified

repair iteration
```

without user manually pointing out the defect.

---

# 182. Functional Preservation Acceptance Test

Redesign working flow.

Expected:

existing functionality remains operational after visual changes.

---

# 183. Design Memory Acceptance Test

Set project rule:

```text
avoid large border radius.
```

Later generate another screen.

Expected:

AgentCode respects project design state without user repeating rule.

---

# 184. Visual Selection Acceptance Test

Where source mapping is supported:

User selects rendered element.

Expected:

AgentCode identifies:

```text
component

source file

related styles
```

and accepts targeted design request.

May be staged later within V1 if framework-specific complexity requires it.

---

# 185. Security UX Acceptance Test

Start Full Security Audit.

Expected UI clearly states:

```text
scope

mode

target

whether active validation is enabled
```

and does not hide risky security actions.

---

# 186. Security Findings UX Acceptance Test

Confirmed vulnerability should display:

```text
severity

confidence

attack path

evidence

recommended fix
```

not merely scanner output.

---

# 187. UI Performance Acceptance Test

While repository indexing and mission execution run:

```text
UI remains responsive.
```

Heavy runtime work does not block rendering thread.

---

# 188. Crash Isolation Acceptance Test

Force renderer/UI crash.

Expected:

```text
Kernel mission continues

UI reconnects after restart.
```

---


# HARDENING NOTE — V1 COMPLETION SCOPING

The original checklist below remains a useful capability checklist. For release-critical interpretation, apply the `REQUIRED_V1`, `REQUIRED_IF_APPLICABLE`, `OPTIONAL_V1`, and `POST_V1` classifications defined in H90. An optional capability must never be represented as implemented merely because its architectural path exists.

---

# 189. V1 Completion Definition

This subsystem is V1-complete only when:

```text
✓ minimal desktop shell exists

✓ project opening works

✓ recent projects work

✓ Goal Mode exists

✓ mission composer exists

✓ mission status view exists

✓ progress derives from Kernel state

✓ fake progress is avoided

✓ user can inspect current task

✓ user can inspect requirement progress

✓ Changes view exists

✓ high-quality diff display exists

✓ Activity view exists

✓ raw detail is progressively disclosed

✓ terminal is available but not primary

✓ model/provider information is inspectable but not dominant

✓ Discuss Mode exists

✓ Discuss Mode is repository-aware

✓ Discuss Mode is read-only by default

✓ discussion can become structured plan

✓ plan can become mission

✓ Design Mode exists

✓ product analysis occurs before design generation

✓ Design Brief exists

✓ project-specific design grammar exists

✓ design tokens can be detected/created

✓ existing design system is inspected

✓ strong existing design can be preserved

✓ anti-AI-slop critic exists

✓ generic design-pattern detection exists

✓ design uniqueness remains usability-aware

✓ screenshot/reference input works

✓ reference analysis works

✓ browser preview works

✓ screenshot feedback loop works

✓ visual QA works

✓ responsive testing works

✓ accessibility checks exist

✓ functional verification occurs after redesign

✓ project design state persists

✓ DESIGN_STATE.md exists

✓ design-state freshness integrates with Doc 02

✓ design skills load progressively

✓ design context is relevance-selected

✓ Security Mode has clear entry points

✓ security scope is visible

✓ findings are presented clearly

✓ attack paths can be inspected

✓ completion notifications work

✓ completion sound works

✓ attention notification differs from completion

✓ routine progress does not spam notifications

✓ closing window does not stop daemon

✓ pause/resume work

✓ mission summary works

✓ advanced context inspection exists

✓ advanced routing inspection exists

✓ worktree complexity is hidden by default

✓ UI consumes structured events rather than every model token

✓ activity compression works

✓ project continues across missions

✓ UI itself supports dark/light/system appearance

✓ UI itself has reasonable accessibility

✓ application remains responsive during background work

✓ user can complete a substantial mission without interacting with raw infrastructure
```

---

# 190. Locked V1 Architectural Principles

The following are locked:

1. AgentCode uses a minimal interface.

2. Internal complexity must not become UI complexity.

3. AgentCode will not use an office/employee/avatar metaphor.

4. Goal Mode is the flagship experience.

5. The user should be able to define a goal and leave.

6. Safe routine coding operations should not require babysitting.

7. Mission progress must derive from real Kernel state.

8. Fake progress is unacceptable.

9. Progressive disclosure is the primary information architecture.

10. Detailed engineering state must remain inspectable.

11. Raw logs are not the default interface.

12. Model/provider selection should be visible only when useful.

13. Discuss Mode is repository-aware rather than generic chat.

14. Discuss Mode is read-only by default.

15. Discussion decisions can become durable mission state.

16. Design Studio is a first-class AgentCode mode.

17. Design generation must begin with product understanding.

18. AgentCode must explicitly fight generic AI UI patterns.

19. Design uniqueness must remain usable and coherent.

20. Product-specific design grammar is preferred over generic templates.

21. Existing good design should be preserved.

22. Existing components should be reused when appropriate.

23. UI implementation must be verified in a real browser.

24. Screenshots are design evidence.

25. Visual models complement deterministic browser checks.

26. Responsive behavior is part of completion.

27. Accessibility is part of design quality.

28. Functional behavior must survive redesign.

29. Project-specific design decisions persist.

30. Design context remains targeted and token-efficient.

31. Security UX must clearly show active-testing scope.

32. Background mission execution must survive window closure.

33. Completion notification should be subtle and informative.

34. Completion sound should be minimal and pleasant.

35. AgentCode should notify only when meaningful.

36. Automatic recoveries should normally remain noninterruptive.

37. UI crashes must not kill missions.

38. The desktop shell should remain lightweight.

39. Tauri 2 + React/TypeScript/Vite is the preferred V1 desktop direction unless extraction/testing reveals a serious blocker.

40. Embedded editors/terminals exist for inspection rather than turning AgentCode into a full VS Code clone.

41. AgentCode should look and feel intentionally designed, not AI-generated.

42. AgentCode itself must satisfy the same design-quality standards that Design Studio applies to user applications.

---

# 191. Final AgentCode Product Surface

```text
                         AGENTCODE
                             │
       ┌─────────────────────┼──────────────────────┐
       ▼                     ▼                      ▼
     GOAL                 DISCUSS                 DESIGN
       │                     │                      │
       │                     │                      │
       │                     └───────┐              │
       │                             │              │
       ▼                             ▼              ▼
AUTONOMOUS MISSION            ENGINEERING      DESIGN STUDIO
       │                      CONVERSATION           │
       │                             │              │
       │                        TURN TO PLAN          │
       │                             │              │
       └───────────────┬─────────────┴──────────────┘
                       ▼
                AUTONOMY KERNEL
                       │
             ┌─────────┼─────────┐
             ▼         ▼         ▼
          CODING   VERIFICATION SECURITY
                       │
                       ▼
                BACKGROUND DAEMON
                       │
                       ▼
                  NOTIFICATION
```

Security remains another primary project capability:

```text
AgentCode
   │
   └── SECURITY
          │
          ├── Quick Audit
          ├── Full Audit
          ├── Cloud Security
          ├── AI Security
          └── Adversarial Validation
```

---

# 192. Design Studio Architecture

```text
                         USER IDEA
                            │
                            ▼
                    PRODUCT UNDERSTANDING
                            │
                            ▼
                       DESIGN BRIEF
                            │
                            ▼
                       DESIGN GRAMMAR
                            │
           ┌────────────────┼─────────────────┐
           ▼                ▼                 ▼
       Typography         Layout          Components
           │                │                 │
           └────────────────┼─────────────────┘
                            ▼
                      UI IMPLEMENTER
                            │
                            ▼
                       LIVE PREVIEW
                            │
                 ┌──────────┼──────────┐
                 ▼          ▼          ▼
               DOM     SCREENSHOT    FUNCTION
                 │          │          │
                 ▼          ▼          ▼
             STRUCTURAL   VISUAL     BROWSER
               QA        CRITIC       QA
                 │          │          │
                 └──────────┼──────────┘
                            ▼
                     ANTI-SLOP REVIEW
                            │
                            ▼
                 RESPONSIVE / ACCESSIBLE
                            │
                           PASS?
                        /         \
                      NO           YES
                      │             │
                    REPAIR      VERIFIED UI
```

---

# 193. AgentCode UI Philosophy

The user should mostly experience:

```text
Goal
  ↓
Working
  ↓
Complete
```

The engineering system underneath may involve:

```text
37 tasks

6 provider switches

3 worktrees

2 context compactions

421 tool calls

14 verification runs
```

but that complexity appears only when inspected.

The product should communicate:

```text
confidence through evidence
```

rather than:

```text
confidence through visual noise.
```

---

# 194. Final Statement

AgentCode should not look powerful because it displays many panels.

It should feel powerful because the user does **not need** many panels.

The default interaction should be:

```text
Tell AgentCode what you want.
```

Then:

```text
leave.
```

If AgentCode can solve the problem itself:

```text
it should not interrupt.
```

If a provider fails:

```text
it should recover.
```

If a model crashes:

```text
it should continue.
```

If something genuinely requires the user:

```text
it should explain exactly why.
```

When the mission finishes:

```text
AgentCode should quietly notify the user,
play a short pleasant completion sound,
and present evidence of what was actually completed.
```

Design Studio follows the same philosophy.

The user should be able to describe an interface at the product level:

```text
Make this look premium, distinctive and appropriate for the product.
```

AgentCode should not respond by generating another generic AI landing page.

Instead it should:

```text
understand the product

derive a design direction

create a coherent visual language

implement it

run the application

inspect it

criticize its own design

repair weak decisions

verify functionality

verify accessibility

verify responsiveness
```

The intended result is:

> **A minimal, quiet and highly capable desktop engineering environment whose complexity remains underneath the surface; whose flagship workflow allows a user to define a goal and walk away; whose Discuss Mode provides repository-aware technical conversation; whose Security Mode makes advanced verification understandable; and whose Design Studio can autonomously create distinctive, product-specific, professional interfaces while actively resisting the generic visual patterns associated with low-quality AI-generated UI.**

This document is the **V1 source of truth for AgentCode's Design Studio and Product UX subsystem.**

