import { globalStyle } from "@vanilla-extract/css";

globalStyle("html", {
  scrollbarGutter: "stable",
  fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif",
});

globalStyle("html > body[data-scroll-locked]", {
  marginRight: "0px !important",
  vars: {
    "--removed-body-scroll-bar-size": "0px !important",
  },
});
