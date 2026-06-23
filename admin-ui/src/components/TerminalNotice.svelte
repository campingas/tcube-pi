<script lang="ts">
  import type { MessageType } from "../types";

  export let type: MessageType = "info";
  export let message: string;
</script>

<section class:error={type === "error"} class:success={type === "success"} class="terminal-notice">
  <span>{type === "error" ? "ERR" : type === "success" ? "OK" : "SYS"}</span>
  <p>{message}</p>
</section>

