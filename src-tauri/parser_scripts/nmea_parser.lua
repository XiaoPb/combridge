-- NMEA Parser Script
-- Parses GPS NMEA protocol data

local parser = {}

parser.name = "NMEA Parser"
parser.description = "Parse GPS NMEA protocol sentences"
parser.author = "ComBridge"
parser.version = "1.0.0"

parser.fields = {
    { key = "latitude", path = "latitude", unit = "°" },
    { key = "longitude", path = "longitude", unit = "°" },
    { key = "altitude", path = "altitude", unit = "m" },
    { key = "speed", path = "speed", unit = "knots" },
    { key = "heading", path = "heading", unit = "°" },
    { key = "satellites", path = "satellites" },
    { key = "hdop", path = "hdop" },
}

local function parse_nmea_coord(coord, dir)
    if not coord or #coord == 0 then
        return 0
    end
    
    local degrees = tonumber(coord:sub(1, 2)) or 0
    local minutes = tonumber(coord:sub(3)) or 0
    local decimal = degrees + minutes / 60
    
    if dir == "S" or dir == "W" then
        decimal = -decimal
    end
    
    return decimal
end

local function parse_gga(fields)
    local result = {}
    
    if #fields >= 2 then
        result.time = fields[2]
    end
    if #fields >= 4 then
        result.latitude = parse_nmea_coord(fields[4], fields[5])
    end
    if #fields >= 6 then
        result.longitude = parse_nmea_coord(fields[6], fields[7])
    end
    if #fields >= 8 then
        result.fix_quality = tonumber(fields[8]) or 0
    end
    if #fields >= 10 then
        result.satellites = tonumber(fields[10]) or 0
    end
    if #fields >= 12 then
        result.hdop = tonumber(fields[12]) or 0
    end
    if #fields >= 14 then
        result.altitude = tonumber(fields[14]) or 0
    end
    
    return result
end

local function parse_rmc(fields)
    local result = {}
    
    if #fields >= 2 then
        result.time = fields[2]
    end
    if #fields >= 4 then
        result.status = fields[4]
    end
    if #fields >= 6 then
        result.latitude = parse_nmea_coord(fields[6], fields[7])
    end
    if #fields >= 8 then
        result.longitude = parse_nmea_coord(fields[8], fields[9])
    end
    if #fields >= 10 then
        result.speed = tonumber(fields[10]) or 0
    end
    if #fields >= 12 then
        result.heading = tonumber(fields[12]) or 0
    end
    if #fields >= 3 then
        result.date = fields[3]
    end
    
    return result
end

function parser.parse(data)
    if not data or #data == 0 then
        return nil
    end
    
    local result = {}
    
    for line in data:gmatch("[^\r\n]+") do
        if line:sub(1, 1) ~= "$" then
            goto continue
        end
        
        local fields = {}
        for field in line:gmatch("[^,]+") do
            table.insert(fields, field)
        end
        
        if #fields < 1 then
            goto continue
        end
        
        local sentence_type = fields[1]:sub(4)
        
        if sentence_type == "GGA" then
            local gga = parse_gga(fields)
            for k, v in pairs(gga) do
                result[k] = v
            end
        elseif sentence_type == "RMC" then
            local rmc = parse_rmc(fields)
            for k, v in pairs(rmc) do
                result[k] = v
            end
        end
        
        ::continue::
    end
    
    return result
end

function parser.validate(data)
    return data ~= nil and #data > 0 and data:find("$", 1, true) ~= nil
end

return parser
