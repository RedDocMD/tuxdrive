require "path"
require "file"
require "file_utils"
require "process"
require "io/memory"
require "colorize"

abstract class FileAction
  abstract def doit
end

class CreateAction < FileAction
  def initialize(path : Path, isDir : Bool)
    @path = path
    @isDir = isDir
  end

  def doit
    if @isDir
      FileUtils.mkdir @path.to_s
    else
      FileUtils.touch @path.to_s
    end
  end
end

class DeleteAction < FileAction
  def initialize(path : Path)
    @path = path
  end

  def doit
    if File.directory? @path
      FileUtils.rm_r @path.to_s
    else
      FileUtils.rm @path.to_s
    end
  end
end

class WriteAction < FileAction
  def initialize(path : Path, str : String)
    @path = path
    @str = str
  end

  def doit
    File.write @path, @str
  end
end

class MoveAction < FileAction
  def initialize(fromPath : Path, toPath : Path)
    @fromPath = fromPath
    @toPath = toPath
  end

  def doit
    FileUtils.mv @fromPath.to_s, @toPath.to_s
  end
end

class WaitAction < FileAction
  def initialize(secs : Int32)
    @secs = secs
  end

  def doit
    sleep @secs
  end
end

class WatchDir < FileAction
  getter path

  def initialize(path : Path)
    @path = path
  end

  def doit
    raise "WatchDir file action cannot be executed"
  end
end

def parse_action(line : String, basedir : Path) : FileAction
  parts = line.split ' '
  case parts[0]
  when "Create"
    isDir = parts[1] == "Dir"
    path = Path.new basedir, parts[2]
    return CreateAction.new path, isDir
  when "Delete"
    path = Path.new basedir, parts[1]
    return DeleteAction.new path
  when "Write"
    path = Path.new basedir, parts[1]
    # Strip leading and tailing double quotes
    str = parts[2][1..-2]
    return WriteAction.new path, str
  when "Move"
    from = Path.new basedir, parts[1]
    to = Path.new basedir, parts[2]
    return MoveAction.new from, to
  when "Wait"
    secs = parts[1].to_i32
    return WaitAction.new secs
  when "WatchDir"
    path = Path.new basedir, parts[1]
    return WatchDir.new path
  else
    raise "#{parts[0]} isn't a valid command"
  end
end

def parse_actions_file(filename : String, basedir : Path) : Array(FileAction)
  lines = File.read_lines filename
  return lines.map { |line| parse_action line, basedir }
end

def run_watcher(watchdirs : Array(Path), outbuf : IO::Memory) : Process
  `cargo build --release --example event_print`
  if ENV.has_key? "CARGO_TARGET_DIR"
    target_dir = ENV["CARGO_TARGET_DIR"]
  else
    target_dir = "."
  end
  args = ["--"] + watchdirs.map { |path| path.to_s }
  proc = Process.new "#{target_dir}/release/examples/event_print", args: args, output: outbuf
  return proc
end

if ARGV.size != 3
  STDERR.puts "Usage: <actionsfile> <resultsfile> <basedir>"
  exit 1
end

basedir = ARGV[2]
actions = parse_actions_file ARGV[0], Path.new basedir

# All paths are relative to base dir

watchdirs = [] of Path
while actions[0].is_a? WatchDir
  first = actions.shift
  watch_path = Path.new first.as(WatchDir).path
  watchdirs << watch_path
  actions.unshift(CreateAction.new watch_path, true)
end

# Start the watcher
watcher_out = IO::Memory.new 1024
watcher = run_watcher watchdirs, watcher_out

# Do the actions
actions.each do |action|
  action.doit
end

# Stop the watcher
watcher.terminate unless watcher.terminated?

watcher_output = watcher_out.each_line.to_a
trimmed_output = watcher_output.map do |line|
  parts = line.split ','
  full_path = Path.new parts[0]
  full_path_parts = full_path.each_part.to_a
  basedir_parts_len = Path.new(basedir).each_part.size
  remaining_parts = full_path_parts[basedir_parts_len..]
  remaining_path = Path.new remaining_parts
  "#{remaining_path},#{parts[1]}"
end
trimmed_output = Set.new trimmed_output

expected_output = Set.new File.read_lines(ARGV[1])

if trimmed_output == expected_output
  puts "PASSED".colorize(:green).mode(:bold).to_s +
       ": Output on runnning #{ARGV[0]} matched output in #{ARGV[1]}."
else
  puts "FAILED".colorize(:red).mode(:bold)
  puts "EXPECTED:".colorize(:red)
  puts expected_output
  puts "OBTAINED:".colorize(:red)
  puts trimmed_output
end
