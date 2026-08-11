const OXLINT_IGNORED = /(^|\/)\.[^/]+\//;

export default {
  '*.{vue,ts,md,yaml,toml,md,json}': 'oxfmt',
  '*.{ts,vue}': (files: string[]) => {
    const lintable = files.filter((file) => !OXLINT_IGNORED.test(file));
    return lintable.length > 0 ? [`oxlint ${lintable.join(' ')}`] : [];
  },
};
