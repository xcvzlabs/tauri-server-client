const PRE_CONNECTION_ROUTE_NAMES = new Set(['index', 'connect']);
const PRE_AUTH_ROUTE_NAMES = new Set(['login', 'register']);

export default defineNuxtRouteMiddleware(async (to) => {
  const session = useSession();
  await session.restoreSession();

  const routeName = typeof to.name === 'string' ? to.name : undefined;

  if (!session.serverUrl.value) {
    return routeName && PRE_CONNECTION_ROUTE_NAMES.has(routeName) ? undefined : navigateTo('/');
  }

  if (!session.isAuthenticated.value) {
    return routeName && PRE_AUTH_ROUTE_NAMES.has(routeName) ? undefined : navigateTo('/login');
  }

  if (
    routeName &&
    (PRE_CONNECTION_ROUTE_NAMES.has(routeName) || PRE_AUTH_ROUTE_NAMES.has(routeName))
  ) {
    return navigateTo('/home');
  }

  return undefined;
});
