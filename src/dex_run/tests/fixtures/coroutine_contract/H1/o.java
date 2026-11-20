package H1;

public final class o {
    public static boolean launched;

    public static void g(androidx.lifecycle.n parent, k context, y1.p block) {
        Scope scope = new Scope();
        launched = Boolean.TRUE.equals(block.d(scope, scope));
    }

    public static Object j(I1.c context, y1.p block, t1.h continuation) {
        Scope scope = new Scope();
        return block.d(scope, scope);
    }
}

final class Scope implements m, r1.d {
    @Override
    public Object i() {
        return "scope-context";
    }
}
