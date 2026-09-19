// @category Guild Wars 2

import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.ArrayDataType;
import ghidra.program.model.data.BooleanDataType;
import ghidra.program.model.data.ByteDataType;
import ghidra.program.model.data.CategoryPath;
import ghidra.program.model.data.CharDataType;
import ghidra.program.model.data.DWordDataType;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.DataTypeConflictHandler;
import ghidra.program.model.data.FloatDataType;
import ghidra.program.model.data.PointerDataType;
import ghidra.program.model.data.QWordDataType;
import ghidra.program.model.data.Structure;
import ghidra.program.model.data.StructureDataType;
import ghidra.program.model.data.VoidDataType;
import ghidra.program.model.data.WideCharDataType;
import ghidra.program.model.data.WordDataType;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.ParameterImpl;
import ghidra.program.model.listing.ReturnParameterImpl;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;
import ghidra.util.xml.XmlUtilities;
import org.jdom2.Document;
import org.jdom2.Element;

import java.io.File;
import java.util.HashMap;
import java.util.Map;

public class ImportProtocols extends GhidraScript {

    private Map<String, DataType> dataTypes;

    @Override
    protected void run() throws Exception {
        StructureDataType point3 = new StructureDataType(CategoryPath.ROOT, "Point3", 0, currentProgram.getDataTypeManager());
        point3.add(FloatDataType.dataType, "x", null);
        point3.add(FloatDataType.dataType, "y", null);
        point3.add(FloatDataType.dataType, "z", null);
        point3.add(DWordDataType.dataType, "unknown", null);

        dataTypes = new HashMap<>();
        dataTypes.put("Byte", ByteDataType.dataType);
        dataTypes.put("Word", WordDataType.dataType);
        dataTypes.put("Dword", DWordDataType.dataType);
        dataTypes.put("Qword", QWordDataType.dataType);
        dataTypes.put("Float", FloatDataType.dataType);
        dataTypes.put("Float2", new ArrayDataType(FloatDataType.dataType, 2, 4));
        dataTypes.put("Float3", new ArrayDataType(FloatDataType.dataType, 3, 4));
        dataTypes.put("Float4", new ArrayDataType(FloatDataType.dataType, 4, 4));
        dataTypes.put("Point3", currentProgram.getDataTypeManager().addDataType(point3, DataTypeConflictHandler.REPLACE_HANDLER));
        dataTypes.put("Guid", new ArrayDataType(ByteDataType.dataType, 16, 1));
        dataTypes.put("Address", new ArrayDataType(ByteDataType.dataType, 28, 1));
        dataTypes.put("String", new PointerDataType(WideCharDataType.dataType, currentProgram.getDataTypeManager()));
        dataTypes.put("CString", new PointerDataType(CharDataType.dataType, currentProgram.getDataTypeManager()));
        dataTypes.put("BufferFixed", new PointerDataType(ByteDataType.dataType, currentProgram.getDataTypeManager()));
        dataTypes.put("BufferVarSmall", new PointerDataType(ByteDataType.dataType, currentProgram.getDataTypeManager()));
        dataTypes.put("BufferVarLarge", new PointerDataType(ByteDataType.dataType, currentProgram.getDataTypeManager()));

        File file = askFile("Select protocols.xml", "Import");

        Document document = XmlUtilities.createSecureSAXBuilder(false, false).build(file);
        for (Element protocol : document.getRootElement().getChildren("Protocol")) {
            for (Element messages : protocol.getChildren("Messages")) {
                String messagesName = messages.getAttributeValue("Name");

                Namespace namespace = currentProgram.getSymbolTable().getOrCreateNameSpace(currentProgram.getGlobalNamespace(), messagesName, SourceType.USER_DEFINED);

                Element server = messages.getChild("Server");
                if (server == null) {
                    continue;
                }

                for (Element message : server.getChildren("Message")) {
                    String messageName = message.getAttributeValue("Name");

                    Structure struct = createStruct("MsgSrv" + messagesName + messageName, message, true);

                    String address = message.getAttributeValue("Addr");
                    if (address == null) {
                        continue;
                    }

                    Function function = getFunctionAt(toAddr(Long.parseUnsignedLong(address.substring(2), 16)));
                    if (function == null) {
                        continue;
                    }

                    function.setParentNamespace(namespace);
                    function.setName("Recv" + messageName, SourceType.USER_DEFINED);
                    function.setReturnType(BooleanDataType.dataType, SourceType.USER_DEFINED);
                    function.updateFunction(
                            null,
                            new ReturnParameterImpl(BooleanDataType.dataType, currentProgram),
                            Function.FunctionUpdateType.DYNAMIC_STORAGE_ALL_PARAMS,
                            true,
                            SourceType.USER_DEFINED,
                            new ParameterImpl("context", new PointerDataType(VoidDataType.dataType, currentProgram.getDataTypeManager()), currentProgram),
                            new ParameterImpl("message", new PointerDataType(struct, currentProgram.getDataTypeManager()), currentProgram)
                    );
                }
            }
        }
    }

    private Structure createStruct(String structureName, Element parent, boolean includeMessageId) {
        StructureDataType struct = new StructureDataType(CategoryPath.ROOT, structureName, 0, currentProgram.getDataTypeManager());

        if (includeMessageId) {
            struct.add(WordDataType.dataType, "messageId", null);
        }

        for (Element field : parent.getChildren()) {
            String fieldType = field.getName();
            String fieldName = field.getAttributeValue("Name");

            DataType dataType;
            switch (fieldType) {
                case "Optional", "ArrayFixed", "ArrayVarSmall", "ArrayVarLarge" ->
                        dataType = new PointerDataType(createStruct(structureName + "_" + fieldName, field, false), currentProgram.getDataTypeManager());
                default -> {
                    dataType = dataTypes.get(fieldType);
                    if (dataType == null) {
                        continue;
                    }
                }
            }

            switch (fieldType) {
                case "ArrayVarSmall" -> struct.add(ByteDataType.dataType, fieldName + "Count", null);
                case "ArrayVarLarge" -> struct.add(WordDataType.dataType, fieldName + "Count", null);
                case "BufferVarSmall" -> struct.add(ByteDataType.dataType, fieldName + "Bytes", null);
                case "BufferVarLarge" -> struct.add(WordDataType.dataType, fieldName + "Bytes", null);
            }

            struct.add(dataType, fieldName, fieldType);
        }

        return (Structure) currentProgram.getDataTypeManager().addDataType(struct, DataTypeConflictHandler.REPLACE_HANDLER);
    }
}
