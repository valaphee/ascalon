// @category Guild Wars 2

import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.*;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.ParameterImpl;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.SymbolTable;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.Node;
import org.w3c.dom.NodeList;

import javax.xml.parsers.DocumentBuilderFactory;
import java.util.Map;

public class MessageTranslator extends GhidraScript {
    private DataTypeManager dataTypeManager;
    private SymbolTable symbolTable;

    private Map<String, DataType> dataTypes;

    @Override
    protected void run() throws Exception {
        dataTypeManager = currentProgram.getDataTypeManager();
        symbolTable = currentProgram.getSymbolTable();

        initializeDataTypes();

        Document document = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(askFile("Select protocols.xml", "Import"));
        Element root = document.getDocumentElement();

        Namespace globalNamespace = currentProgram.getGlobalNamespace();

        NodeList protocols = root.getElementsByTagName("Protocol");
        for (int i = 0; i < protocols.getLength(); i++) {
            Element protocol = (Element) protocols.item(i);

            NodeList messageGroups = protocol.getElementsByTagName("Messages");
            for (int j = 0; j < messageGroups.getLength(); j++) {
                Element messageGroup = (Element) messageGroups.item(j);

                String messageGroupName = messageGroup.getAttribute("Name");

                Namespace messageGroupNamespace = symbolTable.getOrCreateNameSpace(globalNamespace, messageGroupName, SourceType.USER_DEFINED);

                Element recvMessages = (Element) messageGroup.getElementsByTagName("Server").item(0);
                if (recvMessages == null) {
                    continue;
                }

                NodeList messages = recvMessages.getElementsByTagName("Message");
                for (int k = 0; k < messages.getLength(); k++) {
                    Element message = (Element) messages.item(k);

                    String address = message.getAttribute("Addr");
                    if (address.isEmpty()) {
                        continue;
                    }

                    Function function = getFunctionAt(toAddr(Long.parseUnsignedLong(address.substring(2), 16)));
                    if (function == null) {
                        continue;
                    }

                    String messageName = message.getAttribute("Name");

                    function.setParentNamespace(messageGroupNamespace);
                    function.setName("Recv" + messageName, SourceType.USER_DEFINED);
                    function.setReturnType(BooleanDataType.dataType, SourceType.USER_DEFINED);
                    function.updateFunction(null, null, Function.FunctionUpdateType.DYNAMIC_STORAGE_ALL_PARAMS, true, SourceType.USER_DEFINED, new ParameterImpl("context", new PointerDataType(VoidDataType.dataType, dataTypeManager), currentProgram), new ParameterImpl("message", new PointerDataType(createStruct(categoryPath, "MsgSrv" + messageGroupName + messageName, message.getChildNodes(), true), dataTypeManager), currentProgram));
                }
            }
        }
    }

    private void initializeDataTypes() {
        StructureDataType point3 = new StructureDataType(CategoryPath.ROOT, "Point3", 0, dataTypeManager);
        point3.add(FloatDataType.dataType, 4, "x", null);
        point3.add(FloatDataType.dataType, 4, "y", null);
        point3.add(FloatDataType.dataType, 4, "z", null);
        point3.add(DWordDataType.dataType, 4, "unknown", null);
        DataType point3DataType = dataTypeManager.addDataType(point3, DataTypeConflictHandler.REPLACE_HANDLER);

        dataTypes = Map.ofEntries(
                Map.entry("Byte", ByteDataType.dataType),
                Map.entry("Word", WordDataType.dataType),
                Map.entry("Dword", DWordDataType.dataType),
                Map.entry("Qword", QWordDataType.dataType),
                Map.entry("Float", FloatDataType.dataType),
                Map.entry("Float2", new ArrayDataType(FloatDataType.dataType, 2, 4)),
                Map.entry("Float3", new ArrayDataType(FloatDataType.dataType, 3, 4)),
                Map.entry("Float4", new ArrayDataType(FloatDataType.dataType, 4, 4)),
                Map.entry("Point3", point3DataType),
                Map.entry("Guid", new ArrayDataType(ByteDataType.dataType, 16, 1)),
                Map.entry("Address", new ArrayDataType(ByteDataType.dataType, 28, 1)),
                Map.entry("String", new PointerDataType(WideCharDataType.dataType, dataTypeManager)),
                Map.entry("CString", new PointerDataType(CharDataType.dataType, dataTypeManager)),
                Map.entry("BufferFixed", new PointerDataType(ByteDataType.dataType, dataTypeManager)),
                Map.entry("BufferVarSmall", new PointerDataType(ByteDataType.dataType, dataTypeManager)),
                Map.entry("BufferVarLarge", new PointerDataType(ByteDataType.dataType, dataTypeManager))
        );
    }

    private Structure createStruct(String structName, NodeList fields, boolean includeMessageId) {
        StructureDataType struct = new StructureDataType(CategoryPath.ROOT, structName, 0, dataTypeManager);

        if (includeMessageId) {
            struct.add(WordDataType.dataType, 2, "messageId", null);
        }

        for (int i = 0; i < fields.getLength(); i++) {
            Node node = fields.item(i);
            if (!(node instanceof Element field)) {
                continue;
            }

            String fieldType = field.getTagName();
            String fieldName = field.getAttribute("Name");

            DataType dataType;
            if (fieldType.equals("Optional") || fieldType.equals("ArrayFixed") || fieldType.equals("ArrayVarSmall") || fieldType.equals("ArrayVarLarge")) {
                dataType = new PointerDataType(createStruct(structName + "_" + fieldName, field.getChildNodes(), false), dataTypeManager);
            } else {
                dataType = dataTypes.get(fieldType);
                if (dataType == null) {
                    continue;
                }
            }

            if (fieldType.equals("ArrayVarSmall") || fieldType.equals("ArrayVarLarge") || fieldType.equals("BufferVarSmall") || fieldType.equals("BufferVarLarge")) {
                DataType lengthDataType = fieldType.equals("ArrayVarSmall") || fieldType.equals("BufferVarSmall") ? ByteDataType.dataType : WordDataType.dataType;
                struct.add(lengthDataType, lengthDataType.getLength(), fieldName + (fieldType.startsWith("Array") ? "Count" : "Bytes"), null);
            }

            struct.add(dataType, dataType.getLength(), fieldName, field.getTagName());
        }

        return (Structure) dataTypeManager.addDataType(struct, DataTypeConflictHandler.REPLACE_HANDLER);
    }
}
