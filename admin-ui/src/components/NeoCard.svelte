<script lang="ts">
  export let kicker = "";
  export let title = "";
  export let variant: "default" | "array" | "drilldown" = "default";
</script>

<section
  class:button-array={variant === "array"}
  class:drilldown-panel={variant === "drilldown"}
  class="neo-surface neo-card"
>
  {#if kicker || title}
    <div class="section-heading-row">
      <div>
        {#if kicker}<p class="terminal-kicker">{kicker}</p>{/if}
        {#if title}<h2>{title}</h2>{/if}
      </div>
      <slot name="actions" />
    </div>
  {/if}
  <slot />
</section>

