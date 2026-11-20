package winedroid.tests.targets;

public final class ClassCatalog {
    public static Class<?> packagePrivateTarget() {
        return PackagePrivateTarget.class;
    }
}

final class PackagePrivateTarget {
    public PackagePrivateTarget() {
    }
}
