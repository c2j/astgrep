import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

public class ThreadCreationExample {
    
    public void badThreadCreation() {
        // ruleid: CS-MT-01
        Thread thread1 = new Thread();
        thread1.start();
        
        // ruleid: CS-MT-01
        Thread thread2 = new Thread(() -> {
            System.out.println("Running in thread");
        });
        thread2.start();
        
        // ruleid: CS-MT-01
        Thread thread3 = new Thread(new Runnable() {
            @Override
            public void run() {
                System.out.println("Running");
            }
        }, "MyThread");
        thread3.start();
        
        // ruleid: CS-MT-01
        ThreadGroup group = new ThreadGroup("MyGroup");
        Thread thread4 = new Thread(group, () -> {}, "GroupThread");
        thread4.start();
    }
    
    public void goodThreadCreation() {
        // ok: CS-MT-01 - 使用线程池
        ExecutorService executor = Executors.newFixedThreadPool(5);
        executor.submit(() -> {
            System.out.println("Running in thread pool");
        });
        
        // ok: CS-MT-01 - 使用 ThreadPoolExecutor
        ThreadPoolExecutor threadPool = new ThreadPoolExecutor(
            2, 4, 60L, TimeUnit.SECONDS,
            new LinkedBlockingQueue<>(100)
        );
        threadPool.execute(() -> {
            System.out.println("Running in custom thread pool");
        });
        
        // ok: CS-MT-01 - 使用 CompletableFuture
        java.util.concurrent.CompletableFuture.runAsync(() -> {
            System.out.println("Running asynchronously");
        });
    }
}
