[CmdletBinding()]
param(
    [ValidateRange(100, 10000)]
    [int]$Iterations = 1000,
    [ValidateRange(0, 1000)]
    [int]$Warmup = 25
)

$ErrorActionPreference = 'Stop'

# This isolated .NET probe is orchestration only: Rust owns receipt validation
# and percentile calculation. The timed regions are local Named Pipe client
# operations after the matching server is ready; process startup, type
# compilation, server construction, JSON conversion, and output are excluded.
$source = @'
using System;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Threading;

public sealed class HookStatG28NamedPipeResult
{
    public double[] ColdConnectionMs;
    public double[] WarmConnectionMs;
    public double[] WarmOneWayWriteMs;
    public double[] WarmAckRoundTripMs;
}

public static class HookStatG28NamedPipeProbe
{
    private const int FrameBytes = 64;

    public static HookStatG28NamedPipeResult Measure(int iterations, int warmup)
    {
        if (iterations < 1 || warmup < 0)
            throw new ArgumentOutOfRangeException();

        var result = new HookStatG28NamedPipeResult();
        result.ColdConnectionMs = MeasureColdConnections(iterations);
        result.WarmConnectionMs = MeasureWarmConnections(iterations, warmup);
        result.WarmOneWayWriteMs = MeasureSession(iterations, warmup, false);
        result.WarmAckRoundTripMs = MeasureSession(iterations, warmup, true);
        return result;
    }

    // Each cold sample is the first client connection to a newly-created local
    // endpoint. Endpoint creation and worker start are intentionally outside
    // the timed client Connect region.
    private static double[] MeasureColdConnections(int iterations)
    {
        var samples = new double[iterations];
        for (int index = 0; index < iterations; index++)
            samples[index] = ConnectAndExchange();
        return samples;
    }

    // A warm sample reconnects a fresh client to the same long-lived server
    // instance and endpoint name after unmeasured exchanges. This deliberately
    // differs from the cold endpoint construction path above.
    private static double[] MeasureWarmConnections(int iterations, int warmup)
    {
        string name = "hookstat-g28-" + Guid.NewGuid().ToString("N");
        using (var server = new NamedPipeServerStream(
            name,
            PipeDirection.InOut,
            1,
            PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous))
        using (var ready = new ManualResetEventSlim(false))
        {
            Exception serverError = null;
            var worker = new Thread(delegate()
            {
                try
                {
                    ready.Set();
                    for (int index = 0; index < warmup + iterations; index++)
                    {
                        server.WaitForConnection();
                        if (server.ReadByte() < 0)
                            throw new EndOfStreamException();
                        server.WriteByte(0x7f);
                        server.Flush();
                        server.Disconnect();
                    }
                }
                catch (Exception error)
                {
                    serverError = error;
                }
            });
            worker.IsBackground = true;
            worker.Start();
            ready.Wait();

            for (int index = 0; index < warmup; index++)
                ConnectAndExchange(name, false);

            var samples = new double[iterations];
            for (int index = 0; index < iterations; index++)
                samples[index] = ConnectAndExchange(name, true);
            if (!worker.Join(5000) || serverError != null)
                throw new IOException("named pipe warm connection failed");
            return samples;
        }
    }

    private static double ConnectAndExchange()
    {
        string name = "hookstat-g28-" + Guid.NewGuid().ToString("N");
        using (var server = new NamedPipeServerStream(
            name,
            PipeDirection.InOut,
            1,
            PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous))
        using (var ready = new ManualResetEventSlim(false))
        {
            Exception serverError = null;
            var worker = new Thread(delegate()
            {
                try
                {
                    ready.Set();
                    server.WaitForConnection();
                    if (server.ReadByte() < 0)
                        throw new EndOfStreamException();
                    server.WriteByte(0x7f);
                    server.Flush();
                }
                catch (Exception error)
                {
                    serverError = error;
                }
            });
            worker.IsBackground = true;
            worker.Start();
            ready.Wait();

            double elapsed = ConnectAndExchange(name, true);
            if (!worker.Join(5000) || serverError != null)
                throw new IOException("named pipe server exchange failed");
            return elapsed;
        }
    }

    private static double ConnectAndExchange(string name, bool measureConnect)
    {
        using (var client = new NamedPipeClientStream(".", name, PipeDirection.InOut, PipeOptions.None))
        {
            double elapsed = 0.0;
            if (measureConnect)
            {
                var stopwatch = Stopwatch.StartNew();
                client.Connect(5000);
                stopwatch.Stop();
                elapsed = stopwatch.Elapsed.TotalMilliseconds;
            }
            else
            {
                client.Connect(5000);
            }
            client.WriteByte(0x42);
            client.Flush();
            if (client.ReadByte() != 0x7f)
                throw new IOException("pipe acknowledgement mismatch");
            return elapsed;
        }
    }

    private static double[] MeasureSession(int iterations, int warmup, bool acknowledge)
    {
        string name = "hookstat-g28-" + Guid.NewGuid().ToString("N");
        byte[] frame = new byte[FrameBytes];
        for (int index = 0; index < frame.Length; index++)
            frame[index] = (byte)index;

        using (var server = new NamedPipeServerStream(
            name,
            PipeDirection.InOut,
            1,
            PipeTransmissionMode.Byte,
            PipeOptions.Asynchronous))
        using (var ready = new ManualResetEventSlim(false))
        {
            Exception serverError = null;
            var worker = new Thread(delegate()
            {
                try
                {
                    ready.Set();
                    server.WaitForConnection();
                    var received = new byte[FrameBytes];
                    for (int index = 0; index < warmup + iterations; index++)
                    {
                        ReadExactly(server, received);
                        if (acknowledge)
                        {
                            server.WriteByte(0x7f);
                            server.Flush();
                        }
                    }
                }
                catch (Exception error)
                {
                    serverError = error;
                }
            });
            worker.IsBackground = true;
            worker.Start();
            ready.Wait();

            var samples = new double[iterations];
            using (var client = new NamedPipeClientStream(".", name, PipeDirection.InOut, PipeOptions.None))
            {
                client.Connect(5000);
                for (int index = 0; index < warmup; index++)
                {
                    client.Write(frame, 0, frame.Length);
                    client.Flush();
                    if (acknowledge && client.ReadByte() != 0x7f)
                        throw new IOException("pipe acknowledgement mismatch");
                }
                for (int index = 0; index < iterations; index++)
                {
                    var stopwatch = Stopwatch.StartNew();
                    client.Write(frame, 0, frame.Length);
                    client.Flush();
                    if (acknowledge && client.ReadByte() != 0x7f)
                        throw new IOException("pipe acknowledgement mismatch");
                    stopwatch.Stop();
                    samples[index] = stopwatch.Elapsed.TotalMilliseconds;
                }
            }
            if (!worker.Join(5000) || serverError != null)
                throw new IOException("named pipe server session failed");
            return samples;
        }
    }

    private static void ReadExactly(Stream stream, byte[] buffer)
    {
        int offset = 0;
        while (offset < buffer.Length)
        {
            int read = stream.Read(buffer, offset, buffer.Length - offset);
            if (read <= 0)
                throw new EndOfStreamException();
            offset += read;
        }
    }
}
'@

Add-Type -TypeDefinition $source -Language CSharp
$measurement = [HookStatG28NamedPipeProbe]::Measure($Iterations, $Warmup)
[ordered]@{
    schema_version = 1
    cold_connection_ms = @($measurement.ColdConnectionMs)
    warm_connection_ms = @($measurement.WarmConnectionMs)
    warm_one_way_write_ms = @($measurement.WarmOneWayWriteMs)
    warm_ack_round_trip_ms = @($measurement.WarmAckRoundTripMs)
} | ConvertTo-Json -Compress -Depth 3
