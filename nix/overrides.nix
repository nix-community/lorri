self: super: {
  carnix =
    super.carnix.overrideAttrs (old: { patches = old.patches or [] ++ [ ./carnix.patch ]; });
}
