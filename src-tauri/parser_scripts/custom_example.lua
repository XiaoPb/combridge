-- Custom Parser Example
-- Template for creating custom parser scripts
-- Copy this file and modify for your specific data format

local parser = {}

-- Metadata
parser.name = "Custom Parser"
parser.description = "Custom parser template - modify for your needs"
parser.author = "User"
parser.version = "1.0.0"

-- Field definitions (for documentation and UI)
parser.fields = {
    { key = "field1", path = "data.field1", unit = "" },
    { key = "field2", path = "data.field2", unit = "" },
}

-- Main parse function
-- @param data: Raw data string to parse
-- @return: Table with parsed values, or nil if parsing fails
function parser.parse(data)
    -- Example: Parse JSON data
    local success, json_obj = pcall(json.decode, data)
    if not success or type(json_obj) ~= "table" then
        return nil
    end
    
    local result = {}
    
    -- Extract your fields here
    result.field1 = json_obj.data and json_obj.data.field1
    result.field2 = json_obj.data and json_obj.data.field2
    
    return result
end

-- Optional: Validate data before parsing
-- @param data: Raw data string
-- @return: boolean indicating if data is valid
function parser.validate(data)
    return data ~= nil and #data > 0
end

return parser
