package ferrumc.reference;

import java.io.PrintWriter;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.TreeSet;

/** Only specification/float-bit plumbing; numerical answers come from Mojang. */
public final class CubicSplineOracle {
    private static final ClassLoader LOADER = CubicSplineOracle.class.getClassLoader();
    private static Class<?> coordinateType;
    private static Method constant;
    private static Method create;
    private static Method apply;
    private static Method minimum;
    private static Method maximum;

    private record Range(float min, float max) {}
    private record Point(float location, int child, float derivative) {}
    private record Node(Float value, int coordinate, Point[] points) {}
    private record Context(float[] values, List<Integer> trace, int failAt) {}
    private static final class SourceFailure extends RuntimeException {}

    private static final class Case {
        final String name;
        final Range[] ranges;
        final List<Node> nodes = new ArrayList<>();
        final List<float[]> samples = new ArrayList<>();
        int root;
        Case(String name, Range... ranges) { this.name = name; this.ranges = ranges; }
        int constant(float value) {
            nodes.add(new Node(value, -1, null));
            return nodes.size() - 1;
        }
        int multi(int coordinate, Point... points) {
            nodes.add(new Node(null, coordinate, points));
            return nodes.size() - 1;
        }
        Case scalarSamples(TreeSet<Float> values) {
            for (float value : values) samples.add(new float[]{value});
            return this;
        }
    }

    private static String bits(float value) {
        return String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits(value));
    }
    private static Range unlimited() { return new Range(Float.NEGATIVE_INFINITY, Float.POSITIVE_INFINITY); }
    private static Point point(float x, int child, float derivative) { return new Point(x, child, derivative); }
    private static TreeSet<Float> scalarValues(float... boundaries) {
        TreeSet<Float> result = new TreeSet<>();
        for (float value : new float[]{-Float.MAX_VALUE, -10, -4, -2, -1, -0.0f, 0.0f, 1, 2, 4, 10, Float.MAX_VALUE}) result.add(value);
        for (int i = -128; i <= 128; i++) result.add(i / 32.0f);
        for (float value : boundaries) {
            result.add(value);
            if (Float.isFinite(Math.nextDown(value))) result.add(Math.nextDown(value));
            if (Float.isFinite(Math.nextUp(value))) result.add(Math.nextUp(value));
        }
        return result;
    }

    private static List<Case> cases() {
        List<Case> cases = new ArrayList<>();
        for (float value : new float[]{-0.0f, 0.0f, -3.5f, Float.MIN_VALUE, -Float.MIN_VALUE, Float.MAX_VALUE, -Float.MAX_VALUE}) {
            Case c = new Case("constant_" + bits(value));
            c.root = c.constant(value);
            c.samples.add(new float[0]);
            cases.add(c);
        }
        for (float value : new float[]{-0.0f, 0.0f, -2.25f}) {
            for (float derivative : new float[]{-0.0f, 0.0f, -3.0f, 2.0f}) {
                Case c = new Case("one_" + bits(value) + "_" + bits(derivative), unlimited());
                c.root = c.multi(0, point(-0.25f, c.constant(value), derivative));
                cases.add(c.scalarSamples(scalarValues(-0.25f)));
            }
        }
        float[][] derivatives = {{0, 0}, {1, -1}, {-6, 6}, {2, -3}, {-0.0f, 0.0f}};
        for (int i = 0; i < derivatives.length; i++) {
            Case c = new Case("two_" + i, new Range(-2, 2));
            int low = c.constant(-2);
            int high = c.constant(4);
            c.root = c.multi(0, point(-1, low, derivatives[i][0]), point(1, high, derivatives[i][1]));
            cases.add(c.scalarSamples(scalarValues(-1, 1)));
        }
        for (int count : new int[]{6, 64}) {
            Case c = new Case("points_" + count, new Range(-2, 2));
            Point[] points = new Point[count];
            float[] locations = new float[count];
            float[] six = {-1.25f, -0.4f, -0.01f, 0.0f, 0.45f, 1.0f};
            for (int i = 0; i < count; i++) {
                locations[i] = count == 6 ? six[i] : i / 32.0f - 1.0f;
                int child = c.constant(i == 0 ? -0.0f : i % 7 - 3.0f);
                points[i] = point(locations[i], child, (i % 9 - 4) * 0.25f);
            }
            c.root = c.multi(0, points);
            cases.add(c.scalarSamples(scalarValues(locations)));
        }
        for (boolean narrow : new boolean[]{false, true}) {
            Case c = new Case("rounding_" + narrow, new Range(-2, 2));
            float base = 1.0f;
            float next = Math.nextUp(base);
            float end = Math.nextUp(next);
            int a = c.constant(16777216.0f);
            int b = c.constant(-0.125f);
            int d = c.constant(16777218.0f);
            c.root = c.multi(0,
                point(narrow ? base : -0.7f, a, -2.75f),
                point(narrow ? next : 0.1f, b, 3.25f),
                point(narrow ? end : 0.6f, d, -5.5f));
            cases.add(c.scalarSamples(scalarValues(narrow ? base : -0.7f, narrow ? next : 0.1f, narrow ? end : 0.6f)));
        }
        {
            Case c = new Case("subnormal", new Range(-1, 1));
            int a = c.constant(-Float.MIN_VALUE);
            int b = c.constant(Float.MIN_VALUE);
            c.root = c.multi(0, point(-4 * Float.MIN_VALUE, a, 1), point(4 * Float.MIN_VALUE, b, -1));
            TreeSet<Float> samples = scalarValues(-4 * Float.MIN_VALUE, 4 * Float.MIN_VALUE);
            for (int i = -16; i <= 16; i++) samples.add(i * Float.MIN_VALUE);
            cases.add(c.scalarSamples(samples));
        }
        {
            Case c = new Case("signed_zero", unlimited());
            int a = c.constant(0.0f);
            int b = c.constant(-0.0f);
            c.root = c.multi(0, point(-0.0f, a, -0.0f), point(1.0f, b, 0.0f));
            cases.add(c.scalarSamples(scalarValues(-0.0f, 1.0f)));
        }
        {
            Case c = new Case("outside_reported_range", new Range(0, 0));
            int a = c.constant(-2);
            int b = c.constant(4);
            c.root = c.multi(0, point(-1, a, -1), point(1, b, 2));
            cases.add(c.scalarSamples(scalarValues(-1, 1)));
        }
        {
            Case c = new Case("overflow_width", unlimited());
            int a = c.constant(-1);
            int b = c.constant(1);
            c.root = c.multi(0, point(-Float.MAX_VALUE, a, 0), point(Float.MAX_VALUE, b, 0));
            cases.add(c.scalarSamples(scalarValues(-Float.MAX_VALUE, Float.MAX_VALUE)));
        }
        {
            Case c = new Case("overflow_value", unlimited());
            int a = c.constant(-Float.MAX_VALUE);
            int b = c.constant(Float.MAX_VALUE);
            c.root = c.multi(0, point(-1, a, 1), point(1, b, -1));
            cases.add(c.scalarSamples(scalarValues(-1, 1)));
        }
        {
            Case c = new Case("child_overflow", unlimited(), unlimited(), unlimited());
            int value = c.constant(1);
            int left = c.multi(1, point(0, value, Float.MAX_VALUE));
            int right = c.multi(2, point(0, value, 0));
            c.root = c.multi(0, point(0, left, 0), point(1, right, 0));
            c.samples.add(new float[]{0, Float.MAX_VALUE, 0});
            c.samples.add(new float[]{0, 0, 0});
            cases.add(c);
        }
        {
            Case c = new Case("shared_nested", new Range(-3, 3), new Range(-2, 2), new Range(-1, 3));
            int a = c.constant(-2);
            int b = c.constant(4);
            int d = c.constant(-0.0f);
            int e = c.constant(3);
            int left = c.multi(1, point(-1, a, -1), point(1, b, 1));
            int right = c.multi(2, point(0, d, 0), point(2, e, -2));
            c.root = c.multi(0, point(-2, left, 1), point(0, right, 0), point(1, left, -1), point(2, right, 0));
            TreeSet<Float> xs = new TreeSet<>();
            for (int i = -12; i <= 12; i++) xs.add(i / 4.0f);
            for (float x : new float[]{-2, 0, 1, 2}) { xs.add(Math.nextDown(x)); xs.add(Math.nextUp(x)); }
            xs.add(-0.0f);
            for (float x : xs)
                for (float y : new float[]{-3, -1, -0.0f, 0.0f, 1, 3})
                    for (float z : new float[]{-1, 0, 2, 3}) c.samples.add(new float[]{x, y, z});
            cases.add(c);
        }
        {
            Case c = new Case("nan_diagnostics", unlimited(), unlimited());
            int a = c.constant(-2);
            int b = c.constant(4);
            int inner = c.multi(1, point(-1, a, 1), point(1, b, 1));
            c.root = c.multi(0, point(-1, inner, 1), point(1, inner, -1));
            for (float x : new float[]{-4, -1, -0.0f, 0.0f, 0.25f, 1, 4})
                for (float y : new float[]{-3, -1, 0, 1, 3}) c.samples.add(new float[]{x, y});
            cases.add(c);
        }
        {
            Case c = new Case("depth_32", unlimited(), unlimited(), unlimited(), unlimited());
            int child = c.constant(-0.0f);
            for (int i = 0; i < 31; i++) child = c.multi(i % 4, point(0, child, -0.0f));
            c.root = child;
            c.samples.add(new float[]{0, -0.0f, 1, -1});
            c.samples.add(new float[]{-2, 2, -3, 3});
            cases.add(c);
        }
        {
            Case c = new Case("repeated_dag", unlimited(), unlimited());
            int child = c.constant(0.125f);
            for (int i = 0; i < 7; i++) child = c.multi(i % 2,
                point(-1, child, 0), point(0, child, -0.0f), point(1, child, 0));
            c.root = child;
            for (float x : new float[]{-2, -1, -0.0f, 0.0f, 0.25f, 1, 2})
                for (float y : new float[]{-0.0f, 0, 0.5f}) c.samples.add(new float[]{x, y});
            cases.add(c);
        }
        return cases;
    }

    private static Object construct(Case c, int index, Object[] cache, Object[] coordinates) throws Exception {
        if (cache[index] != null) return cache[index];
        Node node = c.nodes.get(index);
        Object actual;
        if (node.value != null) {
            actual = constant.invoke(null, node.value.floatValue());
        } else {
            float[] locations = new float[node.points.length];
            float[] derivatives = new float[node.points.length];
            List<Object> values = new ArrayList<>();
            for (int i = 0; i < node.points.length; i++) {
                Point point = node.points[i];
                locations[i] = point.location;
                derivatives[i] = point.derivative;
                values.add(construct(c, point.child, cache, coordinates));
            }
            actual = create.invoke(null, coordinates[node.coordinate], locations, values, derivatives);
        }
        cache[index] = actual;
        return actual;
    }

    private static String floatList(float[] values) {
        if (values.length == 0) return "-";
        List<String> result = new ArrayList<>();
        for (float value : values) result.add(bits(value));
        return String.join(",", result);
    }
    private static String trace(List<Integer> trace) {
        if (trace.isEmpty()) return "-";
        return String.join(",", trace.stream().map(Object::toString).toList());
    }

    private static void export(Case c, PrintWriter out) throws Exception {
        Object[] coordinates = new Object[c.ranges.length];
        for (int i = 0; i < coordinates.length; i++) {
            final int id = i;
            Range range = c.ranges[i];
            coordinates[i] = Proxy.newProxyInstance(LOADER, new Class<?>[]{coordinateType}, (proxy, method, args) -> {
                return switch (method.getName()) {
                    case "a" -> {
                        Context context = (Context) args[0];
                        context.trace.add(id);
                        if (context.trace.size() == context.failAt) throw new SourceFailure();
                        yield context.values[id];
                    }
                    case "b" -> range.min;
                    case "c" -> range.max;
                    case "toString" -> "fixture-coordinate-" + id;
                    case "hashCode" -> System.identityHashCode(proxy);
                    case "equals" -> proxy == args[0];
                    default -> throw new IllegalArgumentException("unexpected original coordinate method " + method);
                };
            });
            out.println("C|" + c.name + "|" + i + "|" + bits(range.min) + "|" + bits(range.max));
        }
        for (int i = 0; i < c.nodes.size(); i++) {
            Node node = c.nodes.get(i);
            if (node.value != null) {
                out.println("N|" + c.name + "|" + i + "|constant|" + bits(node.value));
            } else {
                List<String> points = new ArrayList<>();
                for (Point point : node.points) points.add(bits(point.location) + "," + point.child + "," + bits(point.derivative));
                out.println("N|" + c.name + "|" + i + "|multi|" + node.coordinate + "|" + String.join(";", points));
            }
        }
        Object actual = construct(c, c.root, new Object[c.nodes.size()], coordinates);
        out.println("D|" + c.name + "|" + c.root + "|" + bits((float) minimum.invoke(actual)) + "|" + bits((float) maximum.invoke(actual)));
        float[] longestFinite = null;
        int longestTrace = 0;
        for (int i = 0; i < c.samples.size(); i++) {
            Context context = new Context(c.samples.get(i), new ArrayList<>(), 0);
            float value = (float) apply.invoke(actual, context);
            out.println((Float.isFinite(value) ? "S" : "X") + "|" + c.name + "|" + i + "|" + floatList(context.values) + "|" + bits(value) + "|" + trace(context.trace));
            if (Float.isFinite(value) && context.trace.size() > longestTrace) {
                longestFinite = context.values;
                longestTrace = context.trace.size();
            }
        }
        TreeSet<Integer> failures = new TreeSet<>();
        for (int failAt = 1; failAt <= Math.min(4, longestTrace); failAt++) failures.add(failAt);
        if (longestTrace > 4) failures.add(longestTrace);
        int index = 0;
        for (int failAt : failures) {
            Context context = new Context(longestFinite, new ArrayList<>(), failAt);
            try {
                apply.invoke(actual, context);
                throw new IllegalStateException("original spline swallowed source failure");
            } catch (InvocationTargetException error) {
                if (!(error.getCause() instanceof SourceFailure)) throw error;
            }
            out.println("E|" + c.name + "|" + index++ + "|" + floatList(context.values) + "|" + failAt + "|" + trace(context.trace));
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("expected output fixture path");
        // Resolve without premature initialization; run the real bootstrap before
        // any original helper initializes Mth or other registry-dependent classes.
        coordinateType = Class.forName("bdh", false, LOADER);
        Class<?> splineType = Class.forName("bbb", false, LOADER);
        Class<?> multipoint = Class.forName("bbb$e", false, LOADER);
        Class.forName("ac").getDeclaredMethod("a").invoke(null);
        Class.forName("amg").getDeclaredMethod("a").invoke(null);
        constant = splineType.getDeclaredMethod("a", float.class);
        create = multipoint.getDeclaredMethod("a", coordinateType, float[].class, List.class, float[].class);
        create.setAccessible(true);
        apply = coordinateType.getDeclaredMethod("a", Object.class);
        minimum = coordinateType.getDeclaredMethod("b");
        maximum = coordinateType.getDeclaredMethod("c");
        try (PrintWriter out = new PrintWriter(Files.newBufferedWriter(Path.of(args[0]), StandardCharsets.UTF_8))) {
            out.println("# Original Minecraft 1.21.8 CubicSpline bbb/bbb$e, direct reflection; f32 raw bits and coordinate request traces");
            for (Case c : cases()) export(c, out);
            if (out.checkError()) throw new java.io.IOException("fixture write failed");
        }
    }
}
