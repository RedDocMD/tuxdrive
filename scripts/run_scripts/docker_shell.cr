require "option_parser"
require "process"
require "file_utils"
require "path"

build = false
shell = false

OptionParser.parse do |parser|
  parser.banner = "Usage: [arguments]"
  parser.on "-b", "--build", "Build the Docker image" do
    build = true
  end
  parser.on "-s", "--shell", "Run Docker image and drop into shell" do
    shell = true
  end
  parser.invalid_option do |option_flag|
    STDERR.puts "ERROR: #{option_flag} is not a valid option."
    STDERR.puts parser
    exit 1
  end
end

if build == shell
  puts "Choose either build or shell option"
  exit 1
end

image_name = "redocmd/tuxdrive"

if build
  comm = "docker"
  args = ["build", "-t", image_name, "."]
  Process.run comm, args, output: STDOUT, error: STDERR
elsif shell
  comm = "docker"
  volume_codedir = "/code"
  volume_cargodir = "/cargodir"

  pwd = FileUtils.pwd
  cargo_dir = "#{Path.home}/.cargo"
  args = ["run", "--rm", "-it",
          "-v", "#{pwd}:#{volume_codedir}",
          "-v", "#{cargo_dir}:#{volume_cargodir}",
          image_name]
  Process.run comm, args, input: STDIN, output: STDOUT, error: STDERR
end
