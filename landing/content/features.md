+++
title = "Features - Nudox"
+++

<section id="hero">

# <span>Encyclopedia</span> for code

<div class="card">

## Make AI code generation reliable by grounding agents in compiler-verified, versioned source-of-truth knowledge.

</div>
</section>

<section>
<div class="aqua-window">

**Compiler-Truth Graph**

```rust
// Query accurate symbol existence
const symbol = await nudox.query("MyComponent", "v2.1.0");

// Returns version-specific API signature
symbol.signature // (props: MyProps) => JSX.Element

// Track lineage and changes
symbol.diff("v2.0.0") // { changed: ["props.variant"] }
```

</div>
</section>

<section id="features-grid">
<div class="card feature-card">

## Symbol Existence

<hr aria-hidden="true">

Know exactly what functions, types, and modules exist in any version of the library. Eliminate hallucinations of non-existent APIs.

</div>
<div class="card feature-card">

## Version Awareness

<hr aria-hidden="true">

Agents code against specific versions, avoiding hallucinations of deprecated or future APIs. Supports both legacy systems and bleeding-edge libraries.

</div>
<div class="card feature-card">

## Lineage & Impact

<hr aria-hidden="true">

Understand how symbols change over time and the impact of those changes across the codebase. Enable safer refactors with queryable lineage.

</div>
</section>

<section>
<div class="card card-centered">

## How It Works

<hr aria-hidden="true">

<ol class="steps-list">
<li class="step-item">
<strong class="step-number">1.</strong>
<div>

**Scan / Ingest**<br>
<span class="step-description">We scan public or private repositories (including enterprise private installs).</span>

</div>
</li>
<li class="step-item">
<strong class="step-number">2.</strong>
<div>

**Compile / Extract**<br>
<span class="step-description">We derive structured representation of symbols and capture relationships (calls, members, visibility).</span>

</div>
</li>
<li class="step-item">
<strong class="step-number">3.</strong>
<div>

**Graph + Query**<br>
<span class="step-description">We store a versioned graph that agents can query via MCP to get compiler-truth facts.</span>

</div>
</li>
<li class="step-item">
<strong class="step-number">4.</strong>
<div>

**Feedback**<br>
<span class="step-description">We track what symbols are queried repeatedly and mismatch points to provide analytics.</span>

</div>
</li>
</ol>

</div>
</section>
