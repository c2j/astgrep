public class ThreadTest {
    public void testThreadStart() {
        // ruleid: java-thread
        Thread thread = new Thread(() -> {
            System.out.println("Thread running");
        });
        thread.start();

        // ruleid: java-thread
        Thread myThread = new Thread();
        myThread.start();

        // ruleid: java-thread
        new Thread(() -> {
            doSomething();
        }).start();
    }

    public void testThreadRun() {
        // OK: java-thread - calling run() directly is not starting a new thread
        Thread thread = new Thread();
        thread.run();
    }

    private void doSomething() {
        System.out.println("Doing something");
    }
}
