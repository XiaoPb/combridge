-- CSV Parser Script
-- Parses comma-separated values data

local parser = {}

parser.name = "CSV Parser"
parser.description = "Parse comma-separated values data"
parser.author = "ComBridge"
parser.version = "1.0.0"

parser.delimiter = ","
parser.has_header = true
parser.fields = {}

function parser.parse(data)
    if not data or #data == 0 then
        return nil
    end
    
    local lines = {}
    for line in data:gmatch("[^\r\n]+") do
        table.insert(lines, line)
    end
    
    if #lines == 0 then
        return nil
    end
    
    local result = {}
    local headers = {}
    local start_idx = 1
    
    if parser.has_header and #lines > 0 then
        local header_line = lines[1]
        for val in header_line:gmatch("[^" .. parser.delimiter .. "]+") do
            table.insert(headers, val:gsub("^%s*(.-)%s*$", "%1"))
        end
    else
        start_idx = 1
        local first_line = lines[1]
        local col_count = 0
        for _ in first_line:gmatch("[^" .. parser.delimiter .. "]+") do
            col_count = col_count + 1
            table.insert(headers, "col_" .. col_count)
        end
    end
    
    for i = start_idx, #lines do
        local line = lines[i]
        local values = {}
        local col_idx = 1
        for val in line:gmatch("[^" .. parser.delimiter .. "]+") do
            local key = headers[col_idx] or ("col_" .. col_idx)
            local num = tonumber(val)
            if num then
                result[key] = num
            end
            col_idx = col_idx + 1
        end
    end
    
    return result
end

function parser.validate(data)
    return data ~= nil and #data > 0
end

return parser
