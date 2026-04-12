-- JSON Parser Script
-- Parses standard JSON format data

local parser = {}

parser.name = "JSON Parser"
parser.description = "Parse standard JSON format data"
parser.author = "ComBridge"
parser.version = "1.0.0"

parser.fields = {}

function parser.parse(data)
    local success, result = pcall(function()
        return json.decode(data)
    end)
    
    if success and type(result) == "table" then
        return result
    end
    return nil
end

function parser.validate(data)
    return data ~= nil and #data > 0
end

return parser
