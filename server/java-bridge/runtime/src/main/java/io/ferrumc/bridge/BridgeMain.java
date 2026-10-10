package io.ferrumc.bridge;

import com.google.gson.JsonObject;
import java.io.*;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;

/** Entry point for the dedicated, trusted-plugin Java process. */
public final class BridgeMain {
    private BridgeMain() {}

    /** Runs the framed stdin/stdout protocol; all diagnostics use stderr. */
    public static void main(String[] args) {
        // Preserve the original stream before plugins can write to System.out.
        OutputStream protocolOutput = new FileOutputStream(FileDescriptor.out);
        System.setOut(new PrintStream(System.err, true, StandardCharsets.UTF_8));
        int exit = 0;
        BridgeEngine engine = null;
        try {
            if (args.length != 0) throw new IllegalArgumentException("bridge accepts no CLI arguments");
            engine = new BridgeEngine();
            JsonObject request;
            while ((request = Frames.read(System.in)) != null) {
                BigInteger id = Requests.id(request);
                JsonObject response = engine.execute(id, request);
                Frames.write(protocolOutput, response);
                if (engine.stopped()) break;
            }
        } catch (Throwable failure) {
            failure.printStackTrace(System.err);
            exit = 1;
        } finally {
            if (engine != null) {
                try { engine.close(); }
                catch (Throwable failure) { failure.printStackTrace(System.err); exit = 1; }
            }
        }
        // A trusted plugin may have started non-daemon threads. Shutdown must
        // still leave Rust with a finished child after onDisable has completed.
        System.exit(exit);
    }
}
