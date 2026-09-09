package cmd

import (
	"fmt"
	"io"
	"os"

	"github.com/spf13/cobra"

	"webtomd/pkg/app"
	"webtomd/pkg/server"
)

type Options struct {
	Name           string
	Strict         bool
	SiteConfigPath string
	Cookie         string
	BrowserProfile string
}

type Runner func(opts Options, url string) error

const missingNameHint = "需要提供 -n"

func Execute() error {
	return NewRootCommand(os.Stdout, os.Stderr).Execute()
}

func NewRootCommand(stdout, stderr io.Writer) *cobra.Command {
	return NewRootCommandWithRunner(stdout, stderr, func(opts Options, url string) error {
		return app.Run(app.Config{
			URL:            url,
			Name:           opts.Name,
			Strict:         opts.Strict,
			SiteConfigPath: opts.SiteConfigPath,
			Cookie:         opts.Cookie,
			BrowserProfile: opts.BrowserProfile,
		})
	})
}

func NewRootCommandWithRunner(stdout, stderr io.Writer, runner Runner) *cobra.Command {
	opts := &Options{}
	cmd := &cobra.Command{
		Use:          "web2md <URL>",
		Short:        "Convert a web article into an offline Markdown note",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			if opts.Name == "" {
				fmt.Fprintf(stderr, "错误：%s 或 --name 指定输出文档名称。\n", missingNameHint)
				cmd.SetOut(stderr)
				_ = cmd.Help()
				return fmt.Errorf("missing required name")
			}
			return runner(*opts, args[0])
		},
	}
	cmd.SetOut(stdout)
	cmd.SetErr(stderr)
	cmd.Flags().StringVarP(&opts.Name, "name", "n", "", "Markdown 文件名，不含 .md 后缀")
	cmd.Flags().BoolVar(&opts.Strict, "strict", false, "资源下载失败时立即返回错误")
	cmd.Flags().StringVar(&opts.SiteConfigPath, "site-config", "", "站点扩展规则 JSON 文件路径")
	cmd.Flags().StringVar(&opts.Cookie, "cookie", "", "请求页面时附加的 Cookie，例如浏览器中复制的 SUB=...; SUBP=...")
	cmd.Flags().StringVar(&opts.BrowserProfile, "browser-profile", "", "浏览器 Profile 目录，用于复制会话后渲染受风控页面")
	cmd.AddCommand(server.NewServeCommand())
	return cmd
}
