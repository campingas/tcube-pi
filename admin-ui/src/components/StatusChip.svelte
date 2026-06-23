<script lang="ts">
  export let label: string;
  export let value: string | boolean | null | undefined;
  export let active = false;

  $: text =
    value === null || value === undefined || value === ""
      ? "Not set"
      : typeof value === "boolean"
        ? value
          ? "Ready"
          : "Missing"
        : String(value);
</script>

<span class:is-active={active} class="connection-status" title={`${label}: ${text}`}>
  <span class="connection-dot"></span>
  <span class="connection-label">{label}</span>
  <strong>{text}</strong>
</span>

