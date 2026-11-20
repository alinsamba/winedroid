package winedroid.tests;

public final class CoroutineContract {
    public static Object runWithContext() {
        return H1.o.j(new I1.c(), new ScopeBlock(), new t1.h());
    }

    public static boolean runLaunch() {
        H1.o.launched = false;
        H1.o.g(new androidx.lifecycle.n(), new H1.k(), new LaunchBlock());
        return H1.o.launched;
    }

    public static Object catchWithContextThrow() {
        try {
            return H1.o.j(new I1.c(), new ThrowBlock(), new t1.h());
        } catch (RuntimeException exception) {
            return "caught-guest-throw";
        }
    }
}

final class ScopeBlock implements y1.p {
    @Override
    public Object d(Object receiver, Object continuation) {
        H1.m scope = (H1.m) receiver;
        r1.d ignored = (r1.d) continuation;
        return receiver == continuation ? scope.i() : "different-coroutine";
    }
}

final class LaunchBlock implements y1.p {
    @Override
    public Object d(Object receiver, Object continuation) {
        H1.m scope = (H1.m) receiver;
        r1.d ignored = (r1.d) continuation;
        return receiver == continuation && "scope-context".equals(scope.i());
    }
}

final class ThrowBlock implements y1.p {
    @Override
    public Object d(Object receiver, Object continuation) {
        throw new RuntimeException("guest coroutine failure");
    }
}
