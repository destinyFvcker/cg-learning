//////////////////////////////////////////////////////////////////////////////
//
//  Triangles.cpp
//
//////////////////////////////////////////////////////////////////////////////

#include "vgl.h"
#include "LoadShaders.h"

enum VAO_IDs { Triangles, NumVAOs };
enum Buffer_IDs { ArrayBuffer, NumBuffers };
enum Attrib_IDs { vPosition = 0 };

GLuint VAOs[NumVAOs];
GLuint Buffers[NumBuffers];

const GLuint NumVertices = 6;

//----------------------------------------------------------------------------
//
// init
//

void init(void)
{

    GLfloat vertices[NumVertices][2] = {
        {-0.90f, -0.90f}, {0.85f, -0.90f}, {-0.90f, 0.85f}, // Triangle 1
        {0.90f, -0.85f},  {0.90f, 0.90f},  {-0.85f, 0.90f}  // Triangle 2
    };

    glGenVertexArrays(NumVAOs, VAOs);
    glCreateBuffers(NumBuffers, Buffers);
    // 下面这两行是OpenGL 4.4引入的，走的是老式OpenGL风格：先把buffer绑定到某个target，然后再对当前绑定的buffer分配storage
    //
    // glBindBuffer(GL_ARRAY_BUFFER, Buffers[ArrayBuffer]);
    // glBufferStorage(GL_ARRAY_BUFFER, sizeof(vertices), vertices, 0);
    glNamedBufferStorage(Buffers[ArrayBuffer], sizeof(vertices), vertices, 0);

    ShaderInfo shaders[] = {{GL_VERTEX_SHADER, "media/shaders/triangles/triangles.vert"},
                            {GL_FRAGMENT_SHADER, "media/shaders/triangles/triangles.frag"},
                            {GL_NONE, NULL}};
    GLuint program = LoadShaders(shaders);
    glUseProgram(program);

    glBindVertexArray(VAOs[Triangles]);
    glBindBuffer(GL_ARRAY_BUFFER, Buffers[ArrayBuffer]);

    glVertexAttribPointer(vPosition, 2, GL_FLOAT, GL_FALSE, 0, BUFFER_OFFSET(0));
    glEnableVertexAttribArray(vPosition);
}

//----------------------------------------------------------------------------
//
// display
//

void display(void)
{
    static const float black[] = {0.0f, 0.0f, 0.0f, 0.0f};

    glClearBufferfv(GL_COLOR, 0, black);

    // 这里确实也可以啥也不加 glBindVertexArray(VAOs[Triangles]);
    glDrawArrays(GL_TRIANGLES, 0, NumVertices);
}

//----------------------------------------------------------------------------
//
// main
//

// Windows GUI 程序使用 WinMain 作为入口，其他平台使用普通的 C/C++ main。
// 下面的程序主体是同一份代码，只是根据平台切换入口函数签名。
#ifdef _WIN32
int CALLBACK WinMain(_In_ HINSTANCE hInstance, _In_ HINSTANCE hPrevInstance, _In_ LPSTR lpCmdLine,
                     _In_ int nCmdShow)
#else
int main(int argc, char** argv)
#endif
{
    glfwInit();

    GLFWwindow* window = glfwCreateWindow(800, 600, "Triangles", NULL, NULL);

    glfwMakeContextCurrent(window);
    gl3wInit();

    init();

    while (!glfwWindowShouldClose(window)) {
        display();
        glfwSwapBuffers(window);
        glfwPollEvents();
    }

    glfwDestroyWindow(window);

    glfwTerminate();
}
