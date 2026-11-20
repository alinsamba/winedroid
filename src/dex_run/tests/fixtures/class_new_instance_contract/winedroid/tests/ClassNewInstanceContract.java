package winedroid.tests;

import winedroid.tests.targets.ClassCatalog;
import winedroid.tests.targets.PrivateConstructorTarget;
import winedroid.tests.targets.PublicInitializedTarget;

public final class ClassNewInstanceContract {
    public static int accessibleConstructorAndInitialization() {
        try {
            return PublicInitializedTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            return -1;
        }
    }

    public static int noZeroArgumentConstructor() {
        try {
            NoZeroArgumentTarget.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 1;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int interfaceType() {
        try {
            ContractInterface.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 2;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int abstractType() {
        try {
            AbstractTarget.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 3;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int arrayType() {
        try {
            String[].class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 4;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int primitiveType() {
        try {
            int.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 5;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int voidType() {
        try {
            void.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 15;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int privateConstructor() {
        try {
            PrivateConstructorTarget.class.newInstance();
            return -1;
        } catch (IllegalAccessException exception) {
            return 6;
        } catch (InstantiationException exception) {
            return -2;
        }
    }

    public static int nonPublicCrossPackageClass() {
        try {
            ClassCatalog.packagePrivateTarget().newInstance();
            return -1;
        } catch (IllegalAccessException exception) {
            return 7;
        } catch (InstantiationException exception) {
            return -2;
        }
    }

    public static int samePackageClassAndConstructor() {
        try {
            return SamePackageTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            return -1;
        }
    }

    public static int privateConstructorFromDeclaringClass() {
        try {
            return PrivateSelfTarget.constructReflectively();
        } catch (ReflectiveOperationException exception) {
            return -1;
        }
    }

    public static int constructorExceptionPropagatesDirectly() {
        try {
            ThrowingConstructor.class.newInstance();
            return -1;
        } catch (ConstructorFailure exception) {
            return 8;
        } catch (ReflectiveOperationException exception) {
            return -2;
        } catch (RuntimeException exception) {
            return -3;
        }
    }

    public static int missingClassForName() {
        try {
            Class.forName("winedroid.tests.DoesNotExist");
            return -1;
        } catch (ClassNotFoundException exception) {
            return 9;
        }
    }

    public static int reflectiveOperationExceptionHierarchy() {
        try {
            NoZeroArgumentTarget.class.newInstance();
            return -1;
        } catch (ReflectiveOperationException exception) {
            return 10;
        }
    }

    public static int emptyStringSpecialCase() {
        try {
            return String.class.newInstance().length() == 0 ? 11 : -1;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int bootstrapNoArgumentConstructors() {
        try {
            Object object = Object.class.newInstance();
            return object != null ? 12 : -1;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int defaultInterfaceInitialization() {
        try {
            DefaultInterfaceTarget.class.newInstance();
            return DefaultInterfaceState.value;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int illegalAccessReflectiveOperationHierarchy() {
        try {
            PrivateConstructorTarget.class.newInstance();
            return -1;
        } catch (ReflectiveOperationException exception) {
            return 13;
        }
    }

    public static int classNotFoundReflectiveOperationHierarchy() {
        try {
            Class.forName("winedroid.tests.StillDoesNotExist");
            return -1;
        } catch (ReflectiveOperationException exception) {
            return 14;
        }
    }

    public static int nullClassReceiver() {
        Class<?> target = null;
        try {
            target.newInstance();
            return -1;
        } catch (NullPointerException exception) {
            return 16;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int bootstrapArrayListConstructor() {
        try {
            java.util.ArrayList<?> list = java.util.ArrayList.class.newInstance();
            return list.isEmpty() ? 17 : -1;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int sourceBackedBootstrapConstructors() {
        try {
            ThreadLocal<?> threadLocal = ThreadLocal.class.newInstance();
            java.util.Date date = java.util.Date.class.newInstance();
            java.util.Random random = java.util.Random.class.newInstance();
            java.io.StringWriter writer = java.io.StringWriter.class.newInstance();
            java.util.concurrent.atomic.AtomicBoolean atomicBoolean =
                    java.util.concurrent.atomic.AtomicBoolean.class.newInstance();
            java.util.concurrent.atomic.AtomicInteger atomicInteger =
                    java.util.concurrent.atomic.AtomicInteger.class.newInstance();
            java.util.concurrent.atomic.AtomicReference<?> atomicReference =
                    java.util.concurrent.atomic.AtomicReference.class.newInstance();
            android.graphics.Rect rect = android.graphics.Rect.class.newInstance();
            android.graphics.RectF rectF = android.graphics.RectF.class.newInstance();
            android.graphics.Matrix matrix = android.graphics.Matrix.class.newInstance();
            android.graphics.Paint paint = android.graphics.Paint.class.newInstance();
            android.graphics.Point point = android.graphics.Point.class.newInstance();
            org.xml.sax.helpers.DefaultHandler handler =
                    org.xml.sax.helpers.DefaultHandler.class.newInstance();
            return threadLocal.get() == null
                    && date.getTime() > 0
                    && random != null
                    && writer.toString().length() == 0
                    && !atomicBoolean.get()
                    && atomicInteger.get() == 0
                    && atomicReference.get() == null
                    && rect.width() == 0
                    && rect.height() == 0
                    && rectF.width() == 0
                    && rectF.height() == 0
                    && matrix != null
                    && paint != null
                    && point.x == 0
                    && point.y == 0
                    && handler != null ? 28 : -1;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int bootstrapAbstractType() {
        try {
            java.util.AbstractList.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 20;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int bootstrapFrameworkAbstractType() {
        try {
            android.text.style.CharacterStyle.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 27;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int bootstrapMissingZeroArgumentConstructor() {
        try {
            Integer.class.newInstance();
            return -1;
        } catch (InstantiationException exception) {
            return 21;
        } catch (IllegalAccessException exception) {
            return -2;
        }
    }

    public static int bootstrapClassForName() {
        try {
            return Class.forName("java.lang.String") == String.class
                    && Class.forName("java.lang.Class") == Class.class
                    && Class.forName("java.lang.Thread") == Thread.class
                    && Class.forName("java.lang.ThreadLocal") == ThreadLocal.class
                    && Class.forName("java.util.Date") == java.util.Date.class
                    && Class.forName("java.util.Random") == java.util.Random.class
                    && Class.forName("java.io.StringWriter") == java.io.StringWriter.class
                    && Class.forName("java.io.ByteArrayOutputStream")
                    == java.io.ByteArrayOutputStream.class ? 18 : -1;
        } catch (ClassNotFoundException exception) {
            return -2;
        }
    }

    public static int classForNameInitializationControl() {
        try {
            ClassLoader loader = ClassNewInstanceContract.class.getClassLoader();
            Class.forName("winedroid.tests.ForNameTarget", false, loader);
            int before = ForNameState.value;
            Class.forName("winedroid.tests.ForNameTarget", true, loader);
            return before == 0 && ForNameState.value == 19 ? 19 : -1;
        } catch (ClassNotFoundException exception) {
            return -2;
        }
    }

    public static int nullClassForName() {
        try {
            Class.forName(null);
            return -1;
        } catch (NullPointerException exception) {
            return 22;
        } catch (ClassNotFoundException exception) {
            return -2;
        }
    }

    public static int classForNameArrayBinaryNames() {
        try {
            return Class.forName("[I") == int[].class
                    && Class.forName("[Ljava.lang.String;") == String[].class ? 24 : -1;
        } catch (ClassNotFoundException exception) {
            return -2;
        }
    }

    public static int classForNameRejectsInvalidAndMissingArrayNames() {
        return classForNameIsMissing("java/lang/String")
                && classForNameIsMissing("Ljava/lang/String;")
                && classForNameIsMissing("[Ljava/lang/String;")
                && classForNameIsMissing("[Lwinedroid.tests.DoesNotExist;") ? 25 : -1;
    }

    public static int nullLoaderUsesBootstrapLoader() {
        try {
            if (Class.forName("java.lang.String", false, null) != String.class) {
                return -1;
            }
            Class.forName("winedroid.tests.ForNameTarget", false, null);
            return -2;
        } catch (ClassNotFoundException exception) {
            return 26;
        }
    }

    private static boolean classForNameIsMissing(String name) {
        try {
            Class.forName(name);
            return false;
        } catch (ClassNotFoundException exception) {
            return true;
        }
    }

    public static int reentrantInitializationOrder() {
        try {
            ReentrantTarget.class.newInstance();
            return ReentrantState.value;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int encodedStaticValuesFollowParentInitialization() {
        try {
            EncodedStaticTarget.class.newInstance();
            return EncodedStaticState.observed == 0 ? 23 : -1;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static int priorStaticUsePreservesParentInitialization() {
        int trigger = PriorStaticTarget.trigger;
        try {
            PriorStaticTarget.class.newInstance();
            return trigger == 0 && PriorStaticState.value == 12 ? 29 : -1;
        } catch (ReflectiveOperationException exception) {
            return -2;
        }
    }

    public static void parkedConstructorCompletes() {
        try {
            ParkedConstructorState.value = ParkedConstructorTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            ParkedConstructorState.value = -1;
        }
    }

    public static void parkedClassInitializationCompletes() {
        try {
            ParkedInitializationState.result =
                    ParkedInitializationTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            ParkedInitializationState.result = -1;
        }
    }

    public static void nestedParkedStaticReadCompletes() {
        try {
            NestedParkedReadState.result =
                    NestedParkedReadTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            NestedParkedReadState.result = -1;
        }
    }

    public static void nestedParkedStaticWriteCompletes() {
        try {
            NestedParkedWriteState.result =
                    NestedParkedWriteTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            NestedParkedWriteState.result = -1;
        }
    }

    public static void nestedParkedStaticReadThrowIsCaught() {
        try {
            NestedParkedThrowState.result =
                    NestedParkedThrowTarget.class.newInstance().value;
        } catch (ReflectiveOperationException exception) {
            NestedParkedThrowState.result = -1;
        }
    }

    public static void nestedParkedInitializerHostError() {
        try {
            NestedParkedHostErrorTarget.class.newInstance();
        } catch (ReflectiveOperationException exception) {
            NestedParkedHostErrorState.result = -1;
        }
    }
}

final class NoZeroArgumentTarget {
    NoZeroArgumentTarget(int value) {
    }
}

interface ContractInterface {
}

abstract class AbstractTarget {
}

final class SamePackageTarget {
    final int value;

    SamePackageTarget() {
        value = 44;
    }
}

final class PrivateSelfTarget {
    private final int value;

    private PrivateSelfTarget() {
        value = 45;
    }

    static int constructReflectively() throws ReflectiveOperationException {
        return PrivateSelfTarget.class.newInstance().value;
    }
}

final class ThrowingConstructor {
    ThrowingConstructor() {
        throw new ConstructorFailure();
    }
}

final class ConstructorFailure extends RuntimeException {
    ConstructorFailure() {
        super("constructor failure");
    }
}

final class DefaultInterfaceState {
    static int value;

    static Object initialize() {
        value = 46;
        return new Object();
    }
}

interface DefaultInitializedInterface {
    Object TOKEN = DefaultInterfaceState.initialize();

    default int defaultValue() {
        return 46;
    }
}

final class DefaultInterfaceTarget implements DefaultInitializedInterface {
}

final class ForNameState {
    static int value;
}

final class ForNameTarget {
    static {
        ForNameState.value = 19;
    }
}

final class ReentrantState {
    static int value;
}

class ReentrantBase {
    static {
        try {
            Class.forName("winedroid.tests.ReentrantTarget");
        } catch (ClassNotFoundException exception) {
            throw new RuntimeException(exception);
        }
        ReentrantState.value = ReentrantState.value * 10 + 1;
    }
}

final class ReentrantTarget extends ReentrantBase {
    static {
        ReentrantState.value = ReentrantState.value * 10 + 2;
    }
}

final class EncodedStaticState {
    static int observed;
}

class EncodedStaticBase {
    static {
        try {
            EncodedStaticState.observed = EncodedStaticTarget.class
                    .getDeclaredField("VALUE")
                    .getInt(null);
        } catch (ReflectiveOperationException exception) {
            EncodedStaticState.observed = -1;
        }
    }
}

final class EncodedStaticTarget extends EncodedStaticBase {
    static final int VALUE = 7;
}

final class PriorStaticState {
    static int value;
}

class PriorStaticBase {
    static {
        PriorStaticState.value = PriorStaticState.value * 10 + 1;
    }
}

final class PriorStaticTarget extends PriorStaticBase {
    static int trigger;

    static {
        PriorStaticState.value = PriorStaticState.value * 10 + 2;
    }
}

final class ParkedConstructorState {
    static int value;
}

final class ParkedConstructorTarget {
    final int value;

    ParkedConstructorTarget() {
        java.util.concurrent.locks.LockSupport.parkNanos(1_000_000L);
        value = 31;
    }
}

final class ParkedInitializationState {
    static int progress;
    static int result;
}

class ParkedInitializationBase {
    static {
        java.util.concurrent.locks.LockSupport.parkNanos(1_000_000L);
        ParkedInitializationState.progress = 3;
    }
}

final class ParkedInitializationTarget extends ParkedInitializationBase {
    static {
        ParkedInitializationState.progress = ParkedInitializationState.progress * 10 + 1;
    }

    final int value;

    ParkedInitializationTarget() {
        value = ParkedInitializationState.progress;
    }
}

final class NestedParkedReadState {
    static int result;
}

final class NestedParkedReadHelper {
    static int value;

    static {
        java.util.concurrent.locks.LockSupport.parkNanos(1_000_000L);
        value = 30;
    }
}

final class NestedParkedReadTarget {
    static int initialized = NestedParkedReadHelper.value + 1;

    final int value;

    NestedParkedReadTarget() {
        value = initialized;
    }
}

final class NestedParkedWriteState {
    static int result;
}

final class NestedParkedWriteHelper {
    static int value;

    static {
        java.util.concurrent.locks.LockSupport.parkNanos(1_000_000L);
        value = 5;
    }
}

final class NestedParkedWriteTarget {
    static {
        NestedParkedWriteHelper.value = 41;
    }

    final int value;

    NestedParkedWriteTarget() {
        value = NestedParkedWriteHelper.value;
    }
}

final class NestedParkedThrowState {
    static int result;
    static int initialized;
}

final class NestedParkedInitializerFailure extends Error {
}

final class NestedParkedThrowHelper {
    static int value = initialize();

    static int initialize() {
        java.util.concurrent.locks.LockSupport.parkNanos(1_000_000L);
        throw new NestedParkedInitializerFailure();
    }
}

final class NestedParkedThrowTarget {
    static {
        try {
            NestedParkedThrowState.initialized = NestedParkedThrowHelper.value;
        } catch (NestedParkedInitializerFailure failure) {
            NestedParkedThrowState.initialized = 43;
        }
    }

    final int value;

    NestedParkedThrowTarget() {
        value = NestedParkedThrowState.initialized;
    }
}

final class NestedParkedHostErrorState {
    static int result;
}

final class NestedParkedHostErrorHelper {
    static int value = initialize();

    static int initialize() {
        java.util.concurrent.locks.LockSupport.parkNanos(1_000_000L);
        missingRuntimePrimitive();
        return 1;
    }

    static native void missingRuntimePrimitive();
}

final class NestedParkedHostErrorTarget {
    static int value = NestedParkedHostErrorHelper.value;
}
