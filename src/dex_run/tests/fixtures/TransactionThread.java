package winedroid.tests;

import android.database.sqlite.SQLiteDatabase;
import java.util.concurrent.locks.LockSupport;

public final class TransactionThread extends Thread {
    public static SQLiteDatabase database;
    public static boolean rejectedBegin;
    public static boolean rejectedBeginAsRuntime;
    public static boolean rejectedSuccess;
    public static boolean sawTransaction;

    @Override
    public void run() {
        sawTransaction = database.inTransaction();
        try {
            database.setTransactionSuccessful();
        } catch (IllegalStateException exception) {
            rejectedSuccess = true;
        }
        try {
            database.beginTransaction();
        } catch (UnsupportedOperationException exception) {
            rejectedBegin = true;
        }
        try {
            database.beginTransaction();
        } catch (RuntimeException exception) {
            rejectedBeginAsRuntime = true;
        }
    }
}

final class MainThreadProbe implements Runnable {
    public static Thread observedThread;

    @Override
    public void run() {
        observedThread = Thread.currentThread();
    }
}

final class ParkedTransactionThread extends Thread {
    public static SQLiteDatabase database;
    public static boolean sawTransactionAfterResume;

    @Override
    public void run() {
        database.beginTransactionNonExclusive();
        database.execSQL("UPDATE test SET num = 3");
        LockSupport.parkNanos(1);
        sawTransactionAfterResume = database.inTransaction();
        database.setTransactionSuccessful();
        database.endTransaction();
    }
}
