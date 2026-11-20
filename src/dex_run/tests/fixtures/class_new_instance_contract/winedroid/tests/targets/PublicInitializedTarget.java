package winedroid.tests.targets;

public final class PublicInitializedTarget extends InitializedBase {
    static {
        InitializationState.value += 1;
    }

    public final int value;

    public PublicInitializedTarget() {
        value = InitializationState.value + 1;
    }
}

class InitializedBase {
    static {
        InitializationState.value = 40;
    }
}

class InitializationState {
    static int value;
}
