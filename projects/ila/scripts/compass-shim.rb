# compass 1.0.3 (the version mathbook-assets was written for) calls
# File.exists? and Dir.exists?, which Ruby 3.2 removed. build_html.py loads
# this file with RUBYOPT=-r before running compass.
class File
  class << self
    alias_method :exists?, :exist? unless respond_to?(:exists?)
  end
end

class Dir
  class << self
    alias_method :exists?, :exist? unless respond_to?(:exists?)
  end
end
