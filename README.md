# Shakedown - Linux Hardware Stress Testing Application

Intended method of use.


```curl -sSL https://git.karner.dev/jdkarner/shakedown/raw/branch/main/scripts/release.sh | bash```

![image](/attachments/9481ceb8-e04a-41e0-8e68-da15cd074a5b)

Shakedown is a GUI application for running comprehensive hardware stress tests on Linux systems to detect hardware faults. It provides a user-friendly interface for configuring and monitoring stress-ng tests while tracking system temperatures and hardware errors.



## Features

- **GUI-based test selection** - Select which subsystems to stress test:
  - CPU - Computation-heavy workloads, FPU, cache stress
  - Memory - RAM integrity, memory subsystem testing
  - Disk - Storage read/write operations
  - I/O - Asynchronous/synchronous I/O operations
  - GPU Burn - CPU compute with CUDA

- **Execution modes**:
  - **Sequential** - Run tests one after another (easier to identify which test causes issues)
  - **Parallel** - Run all tests simultaneously (maximum system stress)
  - **Pre-configured stress-ng jobfiles** - Solid defaults that can be customized if needed

 > Jobfiles are Sequential by default, Parrallel here runs all selected tests at the same time.


- **Real-time monitoring**:
  - Temperature sensors via Tricorder
  - Hardware error logging via Tricorder
  - Test progress and elapsed time

## Building

```scripts/build-dist.sh```
This builds shakedown, stress-ng, and tricorder. It does not currently include building gpu_burn, The forgejo workflow does build gpu_burn.

## Usage

1. Launch the application: (The intended method of use is to use the curl command above.)
   ```bash
   cd $shakedown_dir
   ./shakedown
   ```

2. **Tricorder Monitor**: The Tricorder system monitor will launch automatically in monitor mode. If closed accidentally, click "🖖 Launch Tricorder" in the top bar to relaunch it.

3. **Select Tests**: Check the boxes for the subsystems you want to stress test

4. **Choose Execution Mode**:
   - Sequential: Tests run job at a time
   - Parallel: All selected jobs run simultaneously

5. **Start Tests**: Click "Start Tests" to begin

6. **Monitor**:
   - Use Tricorder for comprehensive real-time monitoring (CPU, GPU, temperatures, fans, logs)
   - Track progress in the status bar

7. **Stop Tests**: Click "Stop Tests" to terminate running tests

## Jobfiles

Pre-configured stress-ng jobfiles are located in the `jobfiles/` directory:

### Customizing Jobfiles

The jobfiles use stress-ng's jobfile format. You can edit them to:
- Adjust test duration (`timeout`)
- Enable/disable specific stressors
- Change resource limits

Example jobfile directive:
```
timeout 300s     # Run for 5 minutes
cpu 0            # Use all CPU cores
verify           # Verify computations for error detection
```


## License

GPLv2

## Acknowledgments

- [stress-ng](https://github.com/ColinIanKing/stress-ng) - The underlying stress testing tool
- [egui](https://github.com/emilk/egui) - Immediate mode GUI library for Rust
- [gpu-burn](https://github.com/wilicc/gpu-burn) - GPU stress testing tool
