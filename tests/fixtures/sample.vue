<template>
  <!-- eslint-disable-next-line vue/no-v-html -->
  <div v-html="content"></div>
  <!-- eslint-disable -->
</template>

<script>
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export default {
  props: {
    content: String,
  },
};
</script>
