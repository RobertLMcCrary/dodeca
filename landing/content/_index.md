+++
title = "NuDox - Truth is timeless"

[extra]
description = "Compiler-backed source-of-truth knowledge for AI agents and developers."
+++

<section id="hero">

# <span>Truth</span> for a truthless age

<div class="card">

## We build an intermediate representation of every codebase, at every point in time, to allow users and agents alike to intelligently query with facts, not summaries

<hr aria-hidden="true">
<div>
<button type="button" class="primary" aria-label="Try NuDox for free">
<span>Try For Free</span>
</button>

<a href="https://dashboard.nudox.org/" role="button" aria-label="Go to dashboard">
<span>Dashboard</span>
</a>

</div>
</div>
</section>

<section id="query-interface">
<div class="aqua-window" id="code-window">

**NuDox Query Engine**

```rust
// Query the source of truth
const schema = await nudox
  .library("react")
  .version("18.2.0")
  .function("useState")
  .get();

// Returns compiler-verified types
schema.params // [initialState: S | (() → S)]
schema.returns // [S, Dispatch SetStateAction<S>>]
```

</div>

<!-- These are my stickies, any dl within this id block is considered a sticky btw -->
<dl>
<dt>The End of Hallucinations</dt>
<dd>

Eliminate hallucinated APls and unreliable documentation. NuDox gives Al agents compiler-accurate knowledge of ANY codebase.

</dd>
</dl>
</section>
