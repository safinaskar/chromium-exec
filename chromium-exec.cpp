/*
{
  "name": "chromium_exec",
  "description": "chromium-exec",
  "path": "/usr/local/bin/chromium-exec",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://aaaaaaaaaaaaa/"]
}
*/

/* { "request": [[<input byte>, <input byte>, ...], <executable>, [<argv[0]>, <argv[1]>, ...]] } */

#include <stdint.h>
#include <sys/wait.h>

#include <string>
#include <string_view>
#include <stdexcept>
#include <exception>
#include <vector>
#include <memory>

#include "libsh-treis/libsh-treis.hpp"

using namespace std::string_literals;

namespace tt = libsh_treis::tools;
namespace tc = libsh_treis::libc;

void
verbatim (std::string_view *view, const std::string &s)
{
  if (!(view->size () >= s.size () && view->compare (0, s.size (), s) == 0))
    {
      throw std::runtime_error ("'"s + s + "' expected"s);
    }

  view->remove_prefix (s.size ());
}

char
one (std::string_view *view)
{
  if (view->empty ())
    {
      throw std::runtime_error ("End of string found");
    }

  char result = view->front ();
  view->remove_prefix (1);
  return result;
}

std::string
json_string (std::string_view *view)
{
  verbatim (view, "\"");

  std::string result;

  for (;;)
    {
      char c = one (view);

      if (c == '\\')
        {
          char c2 = one (view);

          switch (c2)
            {
            case '"':
              result += '"';
              break;
            case '\\':
              result += '\\';
              break;
            case '/':
              result += '/';
              break;
            case 'b':
              result += '\b';
              break;
            case 'f':
              result += '\f';
              break;
            case 'n':
              result += '\n';
              break;
            case 'r':
              result += '\r';
              break;
            case 't':
              result += '\t';
              break;
            case 'u':
              {
                int value = std::stoi (std::string (view->substr (0, 4)), nullptr, 16); // I ignore errors in this line
                view->remove_prefix(4);

                if (value >= 128)
                  {
                    throw std::runtime_error ("Non-ASCII \\uXXXX, this is not supported");
                  }

                result += (char)value;
              }
              break;
            default:
              throw std::runtime_error ("Bad escape");
            }
        }
      else if (c == '"')
        {
          break;
        }
      else
        {
          result += c;
        }
    }

  return result;
}

std::string
to_json_string (const std::string_view &s)
{
  std::string result = "\"";

  for (char c : s)
    {
      if (0x0000 <= c && c < 0x0020)
        {
          result += "\\u"s + tc::x_asprintf ("%04x", (int)c);
        }
      else
        {
          switch (c)
            {
            case '"':
              result += "\\\"";
              break;
            case '\\':
              result += "\\\\";
              break;
            case '\b':
              result += "\\b";
              break;
            case '\f':
              result += "\\f";
              break;
            case '\n':
              result += "\\n";
              break;
            case '\r':
              result += "\\r";
              break;
            case '\t':
              result += "\\t";
              break;
            default:
              result += c;
            }
        }
    }

  return result + '"';
}

void
send (const std::string_view &s)
{
  uint32_t size = s.size ();
  tc::write_repeatedly (1, &size, sizeof (size));
  tc::write_repeatedly (1, s.data (), s.size ());
}

int
main (void)
{
  return libsh_treis::main_helper ([](void){
    std::string output_json;

    try
      {
        std::string input;

        {
          uint32_t size;
          tc::xx_read_repeatedly (0, &size, sizeof (size));

          input = std::string (size, '\0');
          tc::xx_read_repeatedly (0, input.data (), size);
        }

        std::string_view view = input;

        verbatim (&view, "{\"request\":[[");

        std::string std_input;

        if (!(!view.empty () && view.front () == ']'))
          {
            auto iter = view.cbegin ();
            uint8_t current = 0;

            while (*iter != ']')
              {
                if (*iter == ',')
                  {
                    std_input += (char)current;
                    current = 0;
                  }
                else
                  {
                    current *= 10;
                    current += *iter - '0';
                  }

                ++iter;
              }

            std_input += (char)current;
            view.remove_prefix (iter - view.cbegin ());
          }

        verbatim (&view, "],");

        std::string executable = json_string (&view);

        verbatim (&view, ",[");

        std::vector<std::string> args;

        for (;;)
          {
            args.push_back (json_string (&view));

            if (view.empty () || view.front () != ',')
              {
                break;
              }

            verbatim (&view, ",");
          }

        verbatim (&view, "]]}");

        if (!view.empty ())
          {
            throw std::runtime_error ("End of string expected");
          }

        std::unique_ptr<tc::process> child;

        {
          tc::pipe_result child_out_to_parent = tc::x_pipe ();
          tc::pipe_result child_err_to_parent = tc::x_pipe ();

          {
            tc::pipe_result parent_to_child_in = tc::x_pipe ();

            child = std::make_unique<tc::process> ([&](void){
              tc::x_dup2 (parent_to_child_in.readable->get (), 0);
              tc::x_dup2 (child_out_to_parent.writable->get (), 1);
              tc::x_dup2 (child_err_to_parent.writable->get (), 2);

              parent_to_child_in = tc::pipe_result ();
              child_out_to_parent = tc::pipe_result ();
              child_err_to_parent = tc::pipe_result ();

              tc::x_execvp_string (executable.c_str (), args.cbegin (), args.cend ());
            });

            parent_to_child_in.readable = nullptr;
            child_out_to_parent.writable = nullptr;
            child_err_to_parent.writable = nullptr;

            tc::write_repeatedly (parent_to_child_in.writable->get (), std_input.data (), std_input.size ());
          }

          // В идеале нужно слать данные из stdout и stderr в том порядке, в котором они приходят. Но я шлю сперва stdout, а потом stderr. Интерфейс сделан таким, чтобы можно было позже переделать. В частности, расширение должно предполагать, что данные могут идти в любом порядке
          auto pipe_to_json = [](std::unique_ptr<tc::fd> fd, const std::string_view &type){
            for (;;)
              {
                // Лимит, указанный в документации: 1 MB, т. е. 1024 * 1024
                // Нужно:
                // array_size * 4 + 100 <= 1024 * 1024
                // array_size * 4 <= 1024 * 1024 - 100
                // array_size <= (1024 * 1024 - 100)/4
                char pipe_data[(1024 * 1024 - 100)/4];

                auto have_read = tc::read_repeatedly (fd->get (), pipe_data, sizeof (pipe_data));

                if (have_read == 0)
                  {
                    break;
                  }

                std::string num_array;

                char buf[sizeof ("255,")];

                for (int i = 0; i < have_read - 1; ++i)
                  {
                    tc::x_snprintf (buf, sizeof (buf), "%d,", (int)(uint8_t)pipe_data[i]);
                    num_array += buf;
                  }

                num_array += std::to_string ((uint8_t)pipe_data[have_read - 1]);

                send ("{\"type\":"s + to_json_string (type) + ",\"data\":["s + num_array + "]}"s);
              }
          };

          pipe_to_json (std::move (child_out_to_parent.readable), "stdout");
          pipe_to_json (std::move (child_err_to_parent.readable), "stderr");
        }

        int status = tc::x_waitpid_raii (tt::not_null_uptr<tc::process> (std::move (child)), 0);

        if (WIFEXITED (status))
          {
            output_json = "{\"type\":\"terminated\",\"reason\":\"exited\",\"code\":"s + std::to_string (WEXITSTATUS (status)) + "}"s;
          }
        else if (WIFSIGNALED (status))
          {
            output_json = "{\"type\":\"terminated\",\"reason\":\"signaled\",\"signal\":"s + std::to_string (WTERMSIG (status)) + "}"s;
          }
        else
          {
            output_json = "{\"type\":\"terminated\",\"reason\":\"unknown\"}"s;
          }
      }
    catch (const std::exception &e)
      {
        output_json = "{\"type\":\"error\",\"message\":"s + to_json_string (e.what ()) + "}"s;
      }
    catch (...)
      {
        output_json = "{\"type\":\"error\",\"message\":\"Unknown error\"}"s;
      }

    send (output_json);
  });
}
