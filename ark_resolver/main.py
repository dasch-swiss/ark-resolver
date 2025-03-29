import sys
from argparse import ArgumentParser

from ark_resolver.ark import start_server
from ark_resolver.ark_settings import load_settings
from ark_resolver.ark_url import ArkUrlException
from ark_resolver.ark_url import ArkUrlFormatter
from ark_resolver.ark_url import ArkUrlInfo
from ark_resolver.check_digit import CheckDigitException


def main() -> None:
    """
    Main method for app started as CLI
    """
    # parses the command-line arguments
    default_config_path = "ark-config.ini"
    parser = ArgumentParser(description="Convert between DSP resource IRIs and ARK URLs.")
    parser.add_argument("-c", "--config", help="config file (default {})".format(default_config_path))
    group = parser.add_mutually_exclusive_group()
    group.add_argument("-s", "--server", help="start server", action="store_true")
    group.add_argument("-i", "--iri", help="print the converted ARK URL from a given DSP resource IRI (add -v and -d optionally)")
    group.add_argument("-a", "--ark", help="print the converted DSP resource IRI (requires -r) or DSP URL from a given ARK ID")
    parser.add_argument("-r", "--resource", help="generate resource IRI", action="store_true")
    parser.add_argument("-v", "--value", help="value UUID (has to be provided with -i)")
    parser.add_argument("-d", "--date", help="DSP ARK timestamp (has to be provided with -i)")
    args = parser.parse_args()

    # reads the config and registry files
    if args.config is not None:
        config_path = args.config
    else:
        config_path = default_config_path

    try:
        settings = load_settings(config_path)

        if args.server:
            # starts the app as server
            start_server(config_path, settings)
        elif args.iri:
            # prints the converted ARK URL from a given DSP resource IRI
            print(ArkUrlFormatter(settings).resource_iri_to_ark_url(args.iri, args.value, args.date))
        elif args.ark:
            if args.resource:
                # prints the converted DSP resource IRI from a given ARK URL
                print(ArkUrlInfo(settings, args.ark).to_resource_iri())
            else:
                # prints the converted DSP URL from a given ARK URL
                print(ArkUrlInfo(settings, args.ark).to_redirect_url())
        else:
            parser.print_help()
    except ArkUrlException as ex:
        print(ex.message)
        sys.exit(1)
    except CheckDigitException as ex:
        print(ex.message)
        sys.exit(1)


if __name__ == "__main__":
    main()
