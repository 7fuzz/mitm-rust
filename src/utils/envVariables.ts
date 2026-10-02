import type { EnvironmentVariable, VariableVariant } from '../services/tauri/bridge';

/**
 * A variable's `value` always mirrors its active variant. Variables saved before variants
 * existed have none, so they get a single "(auto)" variant holding the plain value.
 */
export const getVariableVariants = (v: EnvironmentVariable): VariableVariant[] => {
  if (v.variants && v.variants.length > 0) {
    return v.variants;
  }
  return [{ name: '(auto)', value: v.value || '' }];
};

export const activeVariantIndex = (v: EnvironmentVariable) => {
  const index = v.activeIndex || 0;
  return index < getVariableVariants(v).length ? index : 0;
};

export const setVariableValue = (v: EnvironmentVariable, value: string): EnvironmentVariable => {
  const active = activeVariantIndex(v);
  return {
    ...v,
    value,
    variants: getVariableVariants(v).map((variant, i) => (i === active ? { ...variant, value } : variant)),
  };
};

export const cycleVariant = (v: EnvironmentVariable, direction: 1 | -1): EnvironmentVariable => {
  const variants = getVariableVariants(v);
  if (variants.length <= 1) return v;
  const next = (activeVariantIndex(v) + direction + variants.length) % variants.length;
  return { ...v, variants, activeIndex: next, value: variants[next].value };
};

export const addVariant = (v: EnvironmentVariable, name: string, value = ''): EnvironmentVariable => {
  const variants = [...getVariableVariants(v), { name, value }];
  return { ...v, variants, activeIndex: variants.length - 1, value };
};

export const renameActiveVariant = (v: EnvironmentVariable, name: string): EnvironmentVariable => {
  const active = activeVariantIndex(v);
  return {
    ...v,
    variants: getVariableVariants(v).map((variant, i) => (i === active ? { ...variant, name } : variant)),
  };
};

export const deleteActiveVariant = (v: EnvironmentVariable): EnvironmentVariable => {
  const variants = getVariableVariants(v);
  if (variants.length <= 1) return v;
  const active = activeVariantIndex(v);
  const remaining = variants.filter((_, i) => i !== active);
  const next = Math.max(0, active - 1);
  return { ...v, variants: remaining, activeIndex: next, value: remaining[next]?.value || '' };
};

export const newVariable = (key = '', value = ''): EnvironmentVariable => ({
  key,
  value,
  enabled: true,
  type: 'default',
  activeIndex: 0,
  variants: [{ name: '(auto)', value }],
});
